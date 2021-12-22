EXE     = Tantabus

ifeq ($(OS),Windows_NT)
	EXE := $(EXE).exe
endif

rule:
	cargo rustc --release -p tantabus-uci -- -C target-cpu=native --emit link=$(EXE)