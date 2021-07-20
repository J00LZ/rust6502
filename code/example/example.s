.autoimport +

.word reset
.include "version.inc"

reset:
  jsr xstuff
yeet:
  lda #$48
  sta $500
  stx $501
  lda #$65
  sta $502
  stx $503
  lda #$6C
  sta $504
  stx $505
  lda #$6C
  sta $506
  stx $507
  lda #$6F
  sta $508
  stx $509
  lda #$2C
  sta $50A
  stx $50B
  lda #$20
  sta $50C
  stx $50D
  lda #$57
  sta $50E
  stx $50F
  lda #$6F
  sta $510
  stx $511
  lda #$72
  sta $512
  stx $513
  lda #$6C
  sta $514
  stx $515
  lda #$64
  sta $516
  stx $517
  lda #$21
  sta $518
  stx $519
  INX
  jmp yeet
loop:
  jmp loop