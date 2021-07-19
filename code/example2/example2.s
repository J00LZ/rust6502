.autoimport +

.word reset

reset:
  lda #$0F
  sta $501
yeet:
  jsr readchar
  AND #$7F
  CLC
  ADC #$40
  STA $500

  jmp yeet
loop:
  jmp loop