EXE     = Tantabus
rule:
	cargo rustc --release -p tantabus-uci -- -C target-cpu=native --emit link=$(EXE)