void reset() {
  char* vga = (char*)0x500;
  char* hello = "Hello, World!";
  int i = 0;
  while ( i < 13)
  {
    vga[i*2] = hello[i];
    vga[i*2+1] = 0x0F;
    i++;
  }
}