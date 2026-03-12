// SPDX-License-Identifier: GPL-2.0
// Copyright (C) 2026 Objective Development Software GmbH

use common::{NanoTime, flow_types::IpAddress};

use crate::{demo_ebpf_proxy::DemoEbpfProxy, nano_time};

impl DemoEbpfProxy {
    #[allow(dead_code)]
    pub fn dump_dns_cache(&self) {
        self.dump_dns_queries();
        self.dump_dns_cnames();
        self.dump_dns_ipv4addr();
        self.dump_dns_ipv6addr();
    }

    pub fn dump_dns_queries(&self) {
        println!("--- Query Cache ---");
        for keyvalue in self.dns_queries.iter() {
            match keyvalue {
                Ok((key, value)) => {
                    println!(
                        "    {}: {}",
                        self.node_manager.node_cache.strings_cache.string_for_identifier(key.name),
                        demo_age(value)
                    );
                }
                Err(err) => {
                    println!("aborting iteration with error: {:?}", err);
                    break;
                }
            }
        }
        println!("--- End Query Cache ---");
    }

    pub fn dump_dns_cnames(&self) {
        println!("--- CNAME Cache ---");
        for keyvalue in self.dns_cnames.iter() {
            match keyvalue {
                Ok((key, value)) => {
                    println!(
                        "    {}: {}",
                        self.node_manager.node_cache.strings_cache.string_for_identifier(key.name),
                        self.node_manager.node_cache.strings_cache.string_for_identifier(value)
                    );
                }
                Err(err) => {
                    println!("aborting iteration with error: {:?}", err);
                    break;
                }
            }
        }
        println!("--- End CNAME Cache ---");
    }

    pub fn dump_dns_ipv4addr(&self) {
        println!("--- IPv4 Cache ---");
        for keyvalue in self.dns_ipv4addr.iter() {
            match keyvalue {
                Ok((key, value)) => {
                    let addr = IpAddress::v4(key.address);
                    println!(
                        "    {:?}: {}",
                        addr,
                        self.node_manager.node_cache.strings_cache.string_for_identifier(value)
                    );
                }
                Err(err) => {
                    println!("aborting iteration with error: {:?}", err);
                    break;
                }
            }
        }
        println!("--- End IPv4 Cache ---");
    }

    pub fn dump_dns_ipv6addr(&self) {
        println!("--- IPv6 Cache ---");
        for keyvalue in self.dns_ipv6addr.iter() {
            match keyvalue {
                Ok((key, value)) => {
                    let addr = IpAddress::v6(key.address);
                    println!(
                        "    {:?}: {}",
                        addr,
                        self.node_manager.node_cache.strings_cache.string_for_identifier(value)
                    );
                }
                Err(err) => {
                    println!("aborting iteration with error: {:?}", err);
                    break;
                }
            }
        }
        println!("--- End IPv6 Cache ---");
    }
}

fn demo_age(time: NanoTime) -> f64 {
    let age = nano_time::now().0 - time.0;
    age as f64 / (1000.0 * 1000.0 * 1000.0)
}
