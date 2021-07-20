.segment "KERNEL"
.export version
reset:
  ldx #$03
  lda version,X
  cmp $8002,X
  bne load_error
  dex
  lda version,X
  cmp $8002,X
  bne load_error
  dex
  lda version,X
  cmp $8002,X
  bne load_error
  dex
  lda version,X
  cmp $8002,X
  bne load_error
  jmp ($8000)

loop:
  jmp loop


load_error:
  ldy #$0
err_loop:
  lda err_str,y
  BEQ loop
  tya
  asl A
  tax
  lda err_str,y
  sta $500,X
  lda #$4F
  sta $501,X
  iny
  jmp err_loop

err_str: .asciiz "Error, invalid executable version!"

.include "version.inc"