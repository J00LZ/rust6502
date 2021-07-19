.segment "KERNEL"
.export xstuff, ystuff, readchar
reset:

  jmp ($8000)

loop:
  jmp loop

xstuff:
  ldx #$00
  rts

ystuff:
  ldy #$00
  rts

readchar:
  lda $10
  BEQ readchar
  CMP #$E0
  BEQ readchar_up
  rts
readchar_up:
  lda $10
  ora #$80 
  rts

test:
  jmp test