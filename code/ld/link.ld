
MEMORY {
    RAM1:
        start $100
        size $400;
    ROM1:
        start $8000
        size $2000;
    KERNEL:
        start $E000
        size $1ff9;
}

SEGMENTS {
    CODE:
        load = ROM1
        type = ro;
    RODATA:
        load = ROM1
        type = ro;
    DATA:
        load = RAM1
        type = rw;
    BSS:
        load = RAM1
        type = bss
        define = yes;
    KERNEL:
        load = KERNEL
        start = $E000
        type = overwrite;
}
