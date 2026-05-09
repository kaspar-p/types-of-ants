PROJECT = ant-naming-domains

-include ../../Common.mk

ifeq ($(ANT_NAMING_DOMAINS_FQDN),)
    $(error ANT_NAMING_DOMAINS_FQDN is not set. Please define it.)
endif

all: release

release: | $(BUILD_OUTPUT_DIR)/$(PROJECT)
	@mkdir -p $(BUILD_OUTPUT_DIR)/config
	ANT_NAMING_DOMAINS_FQDN=$(ANT_NAMING_DOMAINS_FQDN) mo ./config/ddclient.conf.mo > $(BUILD_OUTPUT_DIR)/config/ddclient.conf

.PHONY: all release
