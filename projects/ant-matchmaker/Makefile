PROJECT = ant-matchmaker

CONSUL_VERSION=1.20.0

ifeq ($(PROMETHEUS_OS),)
    $(error PROMETHEUS_OS is not set. Please define it.)
endif

ifeq ($(HASHICORP_ARCH),)
    $(error HASHICORP_ARCH is not set. Please define it.)
endif

ZIPDIR=consul_$(CONSUL_VERSION)_$(PROMETHEUS_OS)_$(HASHICORP_ARCH)
ZIPFILE=$(ZIPDIR).zip

-include ../../Common.mk

all: release

release: $(BUILD_OUTPUT_DIR)/consul $(BUILD_OUTPUT_DIR)/consul.hcl $(BUILD_OUTPUT_DIR)/conf.d
	@echo "built" >> /dev/stderr

clean:
	rm -rf build

build/.cache/:
	@mkdir -p $@

build/.packaging/:
	@mkdir -p $@

build/.packaging/$(ZIPDIR)/: build/.cache/$(ZIPFILE) | build/.packaging/
	@unzip -d $@ -o build/.cache/$(ZIPFILE)

build/.cache/$(ZIPFILE): | build/.cache/
	@cd build/.cache && \
	wget -N https://releases.hashicorp.com/consul/$(CONSUL_VERSION)/$(ZIPFILE)

$(BUILD_OUTPUT_DIR)/consul: | build/.packaging/$(ZIPDIR)/ $(BUILD_OUTPUT_DIR)
	@cp build/.packaging/$(ZIPDIR)/consul $(BUILD_OUTPUT_DIR)/consul

$(BUILD_OUTPUT_DIR)/consul.hcl: consul.hcl | $(BUILD_OUTPUT_DIR)
	@cp consul.hcl $(BUILD_OUTPUT_DIR)/consul.hcl

$(BUILD_OUTPUT_DIR)/conf.d: conf.d | $(BUILD_OUTPUT_DIR)
	@cp -r conf.d $(BUILD_OUTPUT_DIR)/conf.d

.PHONY: all clean release