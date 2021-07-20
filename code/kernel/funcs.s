.export xstuff, ystuff, readchar

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