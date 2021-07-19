
  .org $8000

reset:
  ldx #$00
yeet:
  lda #$48
  sta $400
  stx $401
  lda #$65
  sta $402
  stx $403
  lda #$6C
  sta $404
  stx $405
  lda #$6C
  sta $406
  stx $407
  lda #$6F
  sta $408
  stx $409
  lda #$2C
  sta $40A
  stx $40B
  lda #$20
  sta $40C
  stx $40D
  lda #$57
  sta $40E
  stx $40F
  lda #$6F
  sta $410
  stx $411
  lda #$72
  sta $412
  stx $413
  lda #$6C
  sta $414
  stx $415
  lda #$64
  sta $416
  stx $417
  lda #$21
  sta $418
  stx $419
  INX
  jmp yeet
loop:
  jmp loop

  .org $fffc
  .word reset
  .word $0000