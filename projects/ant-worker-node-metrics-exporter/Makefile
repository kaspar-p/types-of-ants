PROJECT = ant-worker-node-metrics-exporter

NODE_EXPORTER_VERSION=1.10.2

ifeq ($(PROMETHEUS_OS),)
    $(error PROMETHEUS_OS is not set. Please define it.)
endif

ifeq ($(PROMETHEUS_ARCH),)
    $(error PROMETHEUS_ARCH is not set. Please define it.)
endif

ifeq ($(BUILD_OUTPUT_DIR),)
    $(error BUILD_OUTPUT_DIR is not set. Please define it.)
endif

TARDIR=node_exporter-$(NODE_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH)
TARFILE=$(TARDIR).tar.gz

-include ../../Common.mk

all: release

release: $(BUILD_OUTPUT_DIR)/node_exporter
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
	wget -N https://github.com/prometheus/node_exporter/releases/download/v$(NODE_EXPORTER_VERSION)/$(TARFILE)

$(BUILD_OUTPUT_DIR)/node_exporter: | build/.packaging/$(TARDIR) $(BUILD_OUTPUT_DIR)
	@cp build/.packaging/$(TARDIR)/node_exporter $(BUILD_OUTPUT_DIR)/node_exporter

.PHONY: all clean release
