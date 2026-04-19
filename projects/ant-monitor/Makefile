PROJECT = ant-monitor

PROMETHEUS_VERSION=3.11.2

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

release: build/release/prometheus
	@echo "built" >> /dev/stderr

clean:
	rm -rf build

build/.tmp:
	mkdir -p build/.tmp

build/.tmp/prometheus-$(PROMETHEUS_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz: build/.tmp
# For example: https://github.com/prometheus/prometheus/releases/download/v3.11.2/prometheus-3.11.2.darwin-arm64.tar.gz
	@cd build/.tmp && \
	wget https://github.com/prometheus/prometheus/releases/download/v$(PROMETHEUS_VERSION)/prometheus-$(PROMETHEUS_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz \
		-o prometheus-$(PROMETHEUS_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz

build/%/prometheus: build/.tmp build/.tmp/prometheus-$(PROMETHEUS_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz
	@cd build/.tmp && tar -xvz -f prometheus-$(PROMETHEUS_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH).tar.gz.1
	@mkdir -p build/release
	@cp build/.tmp/prometheus-$(PROMETHEUS_VERSION).$(PROMETHEUS_OS)-$(PROMETHEUS_ARCH)/prometheus build/release/prometheus

.PHONY: all clean
