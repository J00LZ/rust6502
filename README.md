# rust6502
> 6502 attempt number 2, this time with memory safety

Thanks kbd-project for the koi8-14.psf file!

Currently, the file `code/example` is loaded when you start the emulator, it'll display "Hello, World!" with 
probably the worst color scheme I've ever seen. 

I used `vasm` to compile the `example.s` file, with this command: `vasm -Fbin -o example -dotdir ./example.s`.

Next on the list of things to add is keyboard handling, which is going to be :sparkles: fun :sparkles: !
