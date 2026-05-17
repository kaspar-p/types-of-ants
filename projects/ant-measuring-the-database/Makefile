PROJECT = ant-measuring-the-database

POSTGRES_EXPORTER_VERSION=0.19.1

ifeq ($(PROMETHEUS_OS),)
    $(error PROMETHEUS_OS is not set. Please define it.)
endif

ifeq ($(PROMETHEUS_ARCH),)
    $(error PROMETHEUS_ARCH is not set. Please define it.)
endif

ifeq ($(BUILD_OUTPUT_DIR),)
    $(error BUILD_OUTPUT_DIR is not set. Please define it.)
endif

TARDIR=postgres_exporter-$(POSTGRES_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH)
TARFILE=$(TARDIR).tar.gz

-include ../../Common.mk

all: release

release: $(BUILD_OUTPUT_DIR)/postgres_exporter $(BUILD_OUTPUT_DIR)/postgres_exporter.yml $(BUILD_OUTPUT_DIR)/postgres_exporter.bin
	@echo "built" >> /dev/stderr

clean:
	rm -rf build

build/.cache/:
	@mkdir -p $@

build/.packaging/:
	@mkdir -p $@

build/.packaging/$(TARDIR)/: build/.cache/$(TARFILE) | build/.packaging/
	@tar -xvz -f build/.cache/$(TARFILE) -C build/.packaging/

build/.cache/$(TARFILE): | build/.cache/
	@cd build/.cache && \
	wget -N https://github.com/prometheus-community/postgres_exporter/releases/download/v$(POSTGRES_EXPORTER_VERSION)/$(TARFILE)

$(BUILD_OUTPUT_DIR)/postgres_exporter.bin: | build/.packaging/$(TARDIR) $(BUILD_OUTPUT_DIR)
	@cp build/.packaging/$(TARDIR)/postgres_exporter $(BUILD_OUTPUT_DIR)/postgres_exporter.bin

$(BUILD_OUTPUT_DIR)/postgres_exporter.yml: | build/.packaging/$(TARDIR) $(BUILD_OUTPUT_DIR)
	@cp src/postgres_exporter.yml $(BUILD_OUTPUT_DIR)/postgres_exporter.yml

$(BUILD_OUTPUT_DIR)/postgres_exporter: | $(BUILD_OUTPUT_DIR)
	@cp src/postgres_exporter $(BUILD_OUTPUT_DIR)/postgres_exporter

.PHONY: all clean release
