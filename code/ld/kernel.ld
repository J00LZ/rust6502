
MEMORY {
    RAM1:
        start $100
        size $500;
    ROM1:
        start $E000
        size $1ff9;
}

SEGMENTS {
    KERNEL:
        load = ROM1
        type=ro;
    RODATA:
        load = ROM1
        type=ro;
    DATA:
        load = RAM1
        type=rw;
    BSS:
        load = RAM1
        type=bss
        define=yes;
}
