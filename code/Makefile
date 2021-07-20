dummy_build_folder := $(shell mkdir -p bin)

KERNEL_SOURCES := $(wildcard kernel/*.s)

KERNEL_OBJECTS := $(patsubst %.s,%.o,$(KERNEL_SOURCES))

%.o: %.s
	ca65 -o $@ $^ -t none --include-dir ./include

%.s: %.c
	cc65 -o $@ $^ -t none

%.o: %.c
	cl65 -o $@ $^ -t none -c

bin/kernel: $(KERNEL_OBJECTS)
	ld65 -o $@ $^ -C ld/kernel.ld

bin/%: %/*.o $(KERNEL_OBJECTS)
	ld65 -o $@ $^ -C ld/link.ld

all: bin/kernel bin/example2

clean: 
	rm ./*/*.o
	rm -rf ./bin