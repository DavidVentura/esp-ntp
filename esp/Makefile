.PHONY: flash flashm build dump_section_size dump_flash_rodata dump_flash_text
export CHRONO_TZ_TIMEZONE_FILTER = (Europe/London|Europe/Amsterdam|UTC)
export ESP_IDF_SYS_ROOT_CRATE = esp
flash:
	cargo espflash flash --release -p /dev/ttyUSB1 -f 80mhz -b 921600 --partition-table partitions.csv
flashm:
	cargo espflash flash --release -p /dev/ttyUSB1 -f 80mhz -b 921600 -M --partition-table partitions.csv
build:
	cargo build --release
