PROJECT = ant-worker-metrics-exporter

NODE_EXPORTER_VERSION=1.10.2

PROMETHEUS_OS ?= $(shell uname)
PROMETHEUS_ARCH ?= $(shell uname -m)

ifeq ($(PROMETHEUS_OS),)
    $(error PROMETHEUS_OS is not set. Please define it.)
endif

ifeq ($(PROMETHEUS_ARCH),)
    $(error PROMETHEUS_ARCH is not set. Please define it.)
endif

-include ../../Makefile.common

all: release

release: build/release/node_exporter
	@echo "built" >> /dev/stderr

clean:
	rm -rf build

build/.tmp:
	mkdir -p build/.tmp

build/.tmp/node_exporter-$(NODE_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz: build/.tmp
	@cd build/.tmp && \
	wget https://github.com/prometheus/node_exporter/releases/download/v$(NODE_EXPORTER_VERSION)/node_exporter-$(NODE_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz \
		-o node_exporter-$(NODE_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz

build/%/node_exporter: build/.tmp build/.tmp/node_exporter-$(NODE_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz
	@cd build/.tmp && tar -xvz -f node_exporter-$(NODE_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz.1
	@mkdir -p build/release
	@cp build/.tmp/node_exporter-$(NODE_EXPORTER_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH)/node_exporter build/release/node_exporter

.PHONY: all clean
