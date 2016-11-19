all:
	cargo build
	sudo setcap cap_setuid,cap_setgid,cap_sys_chroot,cap_sys_admin+ep target/debug/fyc
