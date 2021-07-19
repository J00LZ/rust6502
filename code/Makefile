dummy_build_folder := $(shell mkdir -p bin)

%.o: %.s
	ca65 -o $@ $^ -t none

%.s: %.c
	cc65 -o $@ $^ -t none

%.o: %.c
	cl65 -o $@ $^ -t none -c

bin/kernel: ./kernel/kernel.o
	ld65 -o $@ $^ -C ld/kernel.ld

bin/%: %/*.o ./kernel/kernel.o
	ld65 -o $@ $^ -C ld/link.ld

all: bin/kernel bin/example2

clean: 
	rm ./*/*.o
	rm -rf ./bin