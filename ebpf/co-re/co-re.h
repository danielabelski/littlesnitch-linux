#ifndef __CO_RE_H__
#define __CO_RE_H__

#pragma clang attribute push(__attribute__((preserve_access_index)), apply_to = record)

#define inline __attribute__((always_inline))

// This file declares various Linux kernel structs, stripped down to the fields we acutally need.
// The offset from the struct's base pointer does not matter as it will be fixed by CO-RE
// relocation. Just the field name is important.


typedef unsigned long long __u64;
typedef unsigned int __u32;
typedef __u64 u64;
typedef __u32 u32;

typedef u32 __kernel_dev_t;
typedef __kernel_dev_t dev_t;

typedef unsigned int __kernel_uid32_t;
typedef __kernel_uid32_t uid_t;

typedef unsigned int __kernel_gid32_t;
typedef __kernel_gid32_t gid_t;

typedef int __kernel_pid_t;
typedef __kernel_pid_t pid_t;

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

struct super_block {
	dev_t s_dev;
};

struct inode {
	long unsigned int i_ino;
};

struct dentry {
	struct inode *d_inode;
	struct dentry *d_parent;
	struct qstr d_name;
	struct super_block *d_sb;
};

struct vfsmount {
	struct dentry *mnt_root;
	struct super_block *mnt_sb;
	int mnt_flags;
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
	if (task->cred != 0) {
		return task->cred->uid.val;
	} else {
		return 0;
	}
}

inline pid_t task_struct_tgid(const struct task_struct *task) {
	return task->tgid;
}

inline const struct dentry *dentry_parent(const struct dentry *dentry) {
	return dentry->d_parent;
}

inline const struct qstr *dentry_name(const struct dentry *dentry) {
	return &dentry->d_name;
}

inline long unsigned int dentry_ino(const struct dentry *dentry) {
	return dentry->d_inode->i_ino;
}

inline dev_t dentry_dev(const struct dentry *dentry) {
	return dentry->d_sb->s_dev;
}

inline const struct path *linux_binprm_path(const struct linux_binprm *binprm) {
	return &binprm->file->f_path;
}

inline const struct dentry *path_dentry(const struct path *path) {
	return path->dentry;
}

#pragma clang attribute pop

#endif
