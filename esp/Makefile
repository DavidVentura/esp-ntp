.PHONY: flash flashm build dump_section_size dump_flash_rodata dump_flash_text
flash:
	ESP_IDF_SYS_ROOT_CRATE=esp cargo espflash flash --release -p /dev/ttyUSB1 -f 80mhz -b 921600 --partition-table partitions.csv
flashm:
	ESP_IDF_SYS_ROOT_CRATE=esp cargo espflash flash --release -p /dev/ttyUSB1 -f 80mhz -b 921600 -M --partition-table partitions.csv
build:
	ESP_IDF_SYS_ROOT_CRATE=esp cargo build --release
