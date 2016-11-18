all:
	cargo build
	sudo setcap cap_sys_chroot+ep target/debug/fyc
