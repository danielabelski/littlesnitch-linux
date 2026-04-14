#ifndef __CO_RE_H__
#define __CO_RE_H__

#pragma clang attribute push(__attribute__((preserve_access_index)), apply_to = record)

#define inline __attribute__((always_inline))

#ifndef __bpf__
// only available for BPF compiles:
#define __builtin_preserve_field_info(a, b) 0
#endif

// This file declares various Linux kernel structs, stripped down to the fields we acutally need.
// The offset from the struct's base pointer does not matter as it will be fixed by CO-RE
// relocation. Just the field name is important.


typedef unsigned long long __u64;
typedef unsigned int __u32;
typedef __u64 u64;
typedef __u32 u32;

typedef unsigned int __kernel_uid32_t;
typedef __kernel_uid32_t uid_t;

typedef unsigned int __kernel_gid32_t;
typedef __kernel_gid32_t gid_t;

typedef int __kernel_pid_t;
typedef __kernel_pid_t pid_t;

typedef unsigned long long size_t;

struct qstr {
	union {
		struct {
			u32 hash;
			u32 len;
		};
		u64 hash_len;
	};
	const unsigned char *name;
};


typedef struct {
	gid_t val;
} kgid_t;

typedef struct {
	uid_t val;
} kuid_t;

struct cred {
	kuid_t uid;
	kgid_t gid;
	kuid_t suid;
	kgid_t sgid;
	kuid_t euid;
	kgid_t egid;
	kuid_t fsuid;
	kgid_t fsgid;
};

struct dentry {
	struct dentry *d_parent;
	struct qstr d_name;
};

struct vfsmount {
	struct dentry *mnt_root;
	int mnt_flags;
};

// struct mount is the internal kernel structure that embeds the public struct vfsmount.
// We only declare the fields we need; CO-RE relocation resolves the actual offsets at load time.
struct mount {
	struct mount *mnt_parent;
	struct dentry *mnt_mountpoint;
	struct vfsmount mnt;
	int mnt_id;
};

struct path {
	struct vfsmount *mnt;
	struct dentry *dentry;
};
struct file {
	struct path f_path;
};

struct mm_struct {
	struct {
		struct file *exe_file;
	};
};

struct task_struct {
	struct mm_struct *mm;
	pid_t pid;
	pid_t tgid;
	struct task_struct *real_parent;
	struct task_struct *parent;
	const struct cred *real_cred;
	const struct cred *cred;
};

struct linux_binprm {
	struct mm_struct *mm;
	struct file *executable;
	struct file *interpreter;
	struct file *file;
	struct cred *cred;
	const char *filename;
	const char *interp;
	const char *fdpath;
};

inline const struct path *task_struct_path(const struct task_struct *task) {
	return &task->mm->exe_file->f_path;
}

inline const struct task_struct *task_struct_parent(const struct task_struct *task) {
	return task->parent;
}

inline const struct task_struct *task_struct_real_parent(const struct task_struct *task) {
	return task->real_parent;
}

inline uid_t task_struct_uid(const struct task_struct *task) {
	const struct cred *c = task->cred;
	if (c != 0) {
		return c->uid.val;
	} else {
		return 0;
	}
}

inline pid_t task_struct_tgid(const struct task_struct *task) {
	return task->tgid;
}

inline size_t dentry_name_offset(void) {
	// We use `__builtin_preserve_field_info()`, a special CO-RE compliant offset computation
	// function especially made for eBPF with `BPF_FIELD_BYTE_OFFSET = 0` instead of
	// `__builtin_offsetof()` for both offsets. `__builtin_offsetof()` is not reliably
	// CO-RE-relocated when its result is used in pointer arithmetic.
	// `__builtin_preserve_field_info()` generates a `BPF_CORE_FIELD_BYTE_OFFSET` relocation
	// that survives through arithmetic and is correctly applied by the loader.
	// `__builtin_preserve_field_info()` is only available when targeting BPF.
	return __builtin_preserve_field_info(((struct dentry *)0)->d_name, 0);
}

inline size_t dentry_parent_offset(void) {
	return __builtin_preserve_field_info(((struct dentry *)0)->d_parent, 0);
}

inline size_t vfsmount_root_offset(void) {
	return __builtin_preserve_field_info(((struct vfsmount *)0)->mnt_root, 0);
}

inline size_t mount_vfsmount_offset(void) {
	return __builtin_preserve_field_info(((struct mount *)0)->mnt, 0);
}

inline size_t mount_parent_offset(void) {
	return __builtin_preserve_field_info(((struct mount *)0)->mnt_parent, 0);
}

inline size_t mount_mountpoint_offset(void) {
	return __builtin_preserve_field_info(((struct mount *)0)->mnt_mountpoint, 0);
}

inline const struct path *linux_binprm_path(const struct linux_binprm *binprm) {
	return &binprm->file->f_path;
}

inline const struct dentry *path_dentry(const struct path *path) {
	return path->dentry;
}

inline const struct vfsmount *path_mnt(const struct path *path) {
	return path->mnt;
}

#pragma clang attribute pop
#endif
