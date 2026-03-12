// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use crate::{demo_filter_maps::DemoFilterMaps, demo_node_manager::DemoNodeManager};
use anyhow::Context as _;
use aya::{
    Btf, Ebpf,
    maps::{MapData, RingBuf},
    programs::{
        CgroupSkb, CgroupSkbAttachType, CgroupSock, CgroupSockAddr, FEntry, FExit, TracePoint, links::CgroupAttachMode
    },
};
use aya_log::EbpfLogger;
use common::{
    NanoTime, StringId,
    dns_types::{DnsIpv4Key, DnsIpv6Key, DnsNameKey},
    flow_types::{FlowIdentifier, FlowProperties, SocketProperties},
};
use log::warn;
use std::{path::PathBuf, time::Instant};
use tokio::{self, io::unix::AsyncFd};

/// All communication with the eBPF programs is done via this abstraction.
pub struct DemoEbpfProxy {
    ebpf: Ebpf,
    pub node_manager: DemoNodeManager,
    pub filter_engine: DemoFilterMaps,
    pub active_flows: aya::maps::HashMap<MapData, FlowIdentifier, FlowProperties>, // needs Garbage Collection
    pub socket_properties: aya::maps::HashMap<MapData, u64, SocketProperties>,
    pub dns_queries: aya::maps::HashMap<MapData, DnsNameKey, NanoTime>,
    pub dns_cnames: aya::maps::HashMap<MapData, DnsNameKey, StringId>,
    pub dns_ipv4addr: aya::maps::HashMap<MapData, DnsIpv4Key, StringId>,
    pub dns_ipv6addr: aya::maps::HashMap<MapData, DnsIpv6Key, StringId>,
    pub events: RingBuf<MapData>,
}

impl DemoEbpfProxy {
    pub fn new() -> Self {
        // This will include your eBPF object file as raw bytes at compile-time and load it at
        // runtime. This approach is recommended for most real-world use cases. If you would
        // like to specify the eBPF program at runtime rather than at compile-time, you can
        // reach for `Bpf::load_file` instead.
        let data = aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/linux-snitch-ebpf"));

        // let mut ebpf = aya::EbpfLoader::new()
        //     .btf(Btf::from_sys_fs().ok().as_ref())
        //     .verifier_log_level(VerifierLogLevel::STATS)
        //     .load(data)
        //     .unwrap();
        let mut ebpf = aya::Ebpf::load(data).unwrap();

        let raw_map = ebpf.take_map("ACTIVE_FLOWS").unwrap();
        let active_flows =
            aya::maps::HashMap::<_, FlowIdentifier, FlowProperties>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("SOCKET_PROPERTIES").unwrap();
        let socket_properties =
            aya::maps::HashMap::<_, u64, SocketProperties>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("EVENT_QUEUE").unwrap();
        let events = RingBuf::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("DNS_QUERIES").unwrap();
        let dns_queries = aya::maps::HashMap::<_, DnsNameKey, NanoTime>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("DNS_CNAMES").unwrap();
        let dns_cnames = aya::maps::HashMap::<_, DnsNameKey, StringId>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("DNS_IPV4ADDR").unwrap();
        let dns_ipv4addr =
            aya::maps::HashMap::<_, DnsIpv4Key, StringId>::try_from(raw_map).unwrap();

        let raw_map = ebpf.take_map("DNS_IPV6ADDR").unwrap();
        let dns_ipv6addr =
            aya::maps::HashMap::<_, DnsIpv6Key, StringId>::try_from(raw_map).unwrap();

        let node_manager = DemoNodeManager::new(&mut ebpf);
        let filter_engine = DemoFilterMaps::new(&mut ebpf);
        Self {
            ebpf,
            node_manager,
            filter_engine,
            active_flows,
            socket_properties,
            dns_queries,
            dns_cnames,
            dns_ipv4addr,
            dns_ipv6addr,
            events,
        }
    }

    pub fn start_logger_thread(&mut self) {
        match EbpfLogger::init(&mut self.ebpf) {
            Err(e) => {
                // This can happen if you remove all log statements from your eBPF program.
                warn!("failed to initialize eBPF logger: {e}");
            }
            Ok(logger) => match AsyncFd::with_interest(logger, tokio::io::Interest::READABLE) {
                Ok(mut logger) => {
                    tokio::task::spawn(async move {
                        loop {
                            let mut guard = logger.readable_mut().await.unwrap();
                            guard.get_inner_mut().flush();
                            guard.clear_ready();
                        }
                    });
                }
                Err(e) => {
                    warn!("failed to register for logging FD: {e}");
                }
            },
        }
    }

    pub fn attach(&mut self, cgroup_path: PathBuf) -> anyhow::Result<()> {
        let cgroup = std::fs::File::open(&cgroup_path)
            .with_context(|| format!("{}", cgroup_path.display()))?;

        let programs = [
            ("cgroup_skb_transmit", CgroupSkbAttachType::Egress),
            ("cgroup_skb_receive", CgroupSkbAttachType::Ingress),
        ];
        for (program_name, attach_type) in programs {
            demo_timed("load cgroup_skb program", || {
                let program: &mut CgroupSkb =
                    self.ebpf.program_mut(program_name).unwrap().try_into()?;
                program.load()?;
                program.attach(&cgroup, attach_type, CgroupAttachMode::default())?;
                Ok(())
            })?
        }

        let programs = ["cgroup_sock_create", "cgroup_sock_release"];
        for program_name in programs {
            demo_timed("load cgrouop_sock program", || {
                let program: &mut CgroupSock =
                    self.ebpf.program_mut(program_name).unwrap().try_into()?;
                program.load()?;
                program.attach(&cgroup, CgroupAttachMode::default())?;
                Ok(())
            })?
        }

        let programs = [
            "cgroup_sock_addr_connect4",
            "cgroup_sock_addr_connect6",
            "cgroup_sock_addr_sendmsg4",
            "cgroup_sock_addr_sendmsg6",
        ];
        for program_name in programs {
            demo_timed("load cgrouop_sock_addr program", || {
                let program: &mut CgroupSockAddr =
                    self.ebpf.program_mut(program_name).unwrap().try_into()?;
                program.load()?;
                program.attach(&cgroup, CgroupAttachMode::default())?;
                Ok(())
            })?
        }

        let btf = Btf::from_sys_fs().context("BTF from sysfs")?;

        demo_timed("load bprm_execve entry program", || {
            // This is a BPF_PROG_TYPE_TRACING program of type Fentry, see
            // https://docs.ebpf.io/linux/program-type/BPF_PROG_TYPE_TRACING/
            // check for availability with
            // bpftool btf dump file /sys/kernel/btf/vmlinux | grep 'FUNC ' | grep -i binprm
            // and
            // sudo grep -i binprm /sys/kernel/debug/tracing/available_filter_functions
            // Kernel must be compiled with CONFIG_DEBUG_INFO_BTF=y and CONFIG_FUNCTION_TRACER=y
            let program: &mut FEntry =
                self.ebpf.program_mut("fentry_bprm_execve").unwrap().try_into()?;
            program.load("bprm_execve", &btf)?;
            program.attach()?;
            Ok(())
        })?;

        demo_timed("load bprm_execve exit program", || {
            let program: &mut FExit =
                self.ebpf.program_mut("fexit_bprm_execve").unwrap().try_into()?;
            program.load("bprm_execve", &btf)?;
            program.attach()?;
            Ok(())
        })?;

        demo_timed("load sched_process_exec program", || {
            let program: &mut TracePoint =
                self.ebpf.program_mut("tracepoint_sched_process_exec").unwrap().try_into()?;
            program.load()?;
            program.attach("sched", "sched_process_exec")?;
            Ok(())
        })?;

        demo_timed("load sched_process_fork program", || {
            let program: &mut TracePoint =
                self.ebpf.program_mut("tracepoint_sched_process_fork").unwrap().try_into()?;
            program.load()?;
            program.attach("sched", "sched_process_fork")?;
            Ok(())
        })?;

        demo_timed("load sched_process_exit program", || {
            let program: &mut TracePoint =
                self.ebpf.program_mut("tracepoint_sched_process_exit").unwrap().try_into()?;
            program.load()?;
            program.attach("sched", "sched_process_exit")?;
            Ok(())
        })?;

        Ok(())
    }
}

fn demo_timed(what: &str, block: impl FnOnce() -> anyhow::Result<()>) -> anyhow::Result<()> {
    let start = Instant::now();
    let result = block();
    println!("{} took {:?}", what, start.elapsed());
    result
}
