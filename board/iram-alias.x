/* ESP32-S3 integration for esp-hal 1.2.2 (MIT OR Apache-2.0).
 * Reuse the HAL's memory map and section definitions, with one change to
 * esp32s3.x: reserve the full vector region in the shared SRAM alias. */
INCLUDE "memory.x"
INCLUDE "alias.x"
INCLUDE "exception.x"

/* Flash data and instructions share the same physical mapping. */
SECTIONS {
  .rotext_dummy (NOLOAD) : {
    . = ALIGN(ALIGNOF(.rodata));
    . = ALIGN(ALIGNOF(.rodata.wifi));
    . = . + SIZEOF(.flash.appdesc);
    . = . + SIZEOF(.rodata);
    . = . + SIZEOF(.rodata.wifi);
    . = ALIGN(0x10000) + 0x20;
    _rotext_reserved_start = .;
  } > ROTEXT
}
INSERT BEFORE .text;

/* GNU ld 2.35 can evaluate SIZEOF(.vectors) as zero at this insertion.
 * Reserve its configured extent before writing data through the DRAM alias. */
SECTIONS {
  .rwdata_dummy (NOLOAD) : ALIGN(4) {
    . = . + SIZEOF(.rwtext) + SIZEOF(.rwtext.wifi) + VECTORS_SIZE;
  } > RWDATA
}
INSERT BEFORE .data;

SECTIONS {
  INCLUDE "rwtext.x"
  INCLUDE "rwdata.x"
}
INCLUDE "rodata.x"
INCLUDE "text.x"
INCLUDE "rtc_fast.x"
INCLUDE "rtc_slow.x"
INCLUDE "stack.x"
INCLUDE "dram2.x"
INCLUDE "dcache_reclaimed.x"
INCLUDE "metadata.x"
INCLUDE "eh_frame.x"
INCLUDE "hal-defaults.x"

ASSERT(ADDR(.data) >= ORIGIN(RWDATA) + VECTORS_SIZE + SIZEOF(.rwtext) + SIZEOF(.rwtext.wifi),
       "ESP32-S3 instruction RAM overlaps writable data");
