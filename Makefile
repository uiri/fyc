all:
	cargo build
	sudo setcap cap_setuid,cap_setgid,cap_sys_chroot+ep target/debug/fyc
