PROJECT = ant-monitor

PROMETHEUS_VERSION=3.11.2

ifeq ($(PROMETHEUS_OS),)
    $(error PROMETHEUS_OS is not set. Please define it.)
endif

ifeq ($(PROMETHEUS_ARCH),)
    $(error PROMETHEUS_ARCH is not set. Please define it.)
endif

ifeq ($(BUILD_OUTPUT_DIR),)
    $(error PROMETHEUS_ARCH is not set. Please define it.)
endif

TARDIR=prometheus-$(PROMETHEUS_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH)
TARFILE=$(TARDIR).tar.gz

-include ../../Common.mk

all: release

release: $(BUILD_OUTPUT_DIR)/prometheus
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
# For example: https://github.com/prometheus/prometheus/releases/download/v3.11.2/prometheus-3.11.2.darwin-arm64.tar.gz
	@cd build/.cache && \
	wget -N https://github.com/prometheus/prometheus/releases/download/v$(PROMETHEUS_VERSION)/$(TARFILE)

$(BUILD_OUTPUT_DIR)/prometheus: $(BUILD_OUTPUT_DIR)/prometheus.yml | $(BUILD_OUTPUT_DIR) build/.packaging/$(TARDIR)
	@cp build/.packaging/$(TARDIR)/prometheus $(BUILD_OUTPUT_DIR)/prometheus

$(BUILD_OUTPUT_DIR)/prometheus.yml: prometheus.yml | $(BUILD_OUTPUT_DIR)
	@cp prometheus.yml $(BUILD_OUTPUT_DIR)/prometheus.yml

.PHONY: all clean release
