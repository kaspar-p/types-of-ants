PROJECT = ant-just-checking-in

BLACKBOX_EXPORTER_VERSION=0.28.0

ifeq ($(PROMETHEUS_OS),)
    $(error PROMETHEUS_OS is not set. Please define it.)
endif

ifeq ($(PROMETHEUS_ARCH),)
    $(error PROMETHEUS_ARCH is not set. Please define it.)
endif

TARDIR=blackbox_exporter-$(BLACKBOX_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH)
TARFILE=$(TARDIR).tar.gz

-include ../../Common.mk

all: release

release: $(BUILD_OUTPUT_DIR)/blackbox_exporter $(BUILD_OUTPUT_DIR)/blackbox.yml
	@echo "built" >> /dev/stderr

clean:
	rm -rf build

build/.cache/:
	@mkdir -p $@

build/.packaging/:
	@mkdir -p $@

build/.packaging/$(TARDIR)/: build/.cache/$(TARFILE) | build/.packaging/
	@tar -xvz -f build/.cache/$(TARFILE) -C build/.packaging/

build/.cache/$(TARFILE): | build/.cache
	@cd build/.cache && \
	wget -N https://github.com/prometheus/blackbox_exporter/releases/download/v$(BLACKBOX_EXPORTER_VERSION)/$(TARFILE)

$(BUILD_OUTPUT_DIR)/blackbox_exporter: | $(BUILD_OUTPUT_DIR) build/.packaging/$(TARDIR)
	@cp build/.packaging/$(TARDIR)/blackbox_exporter $@

$(BUILD_OUTPUT_DIR)/blackbox.yml: blackbox.yml | $(BUILD_OUTPUT_DIR)
	@cp $< $@

.PHONY: all clean release
