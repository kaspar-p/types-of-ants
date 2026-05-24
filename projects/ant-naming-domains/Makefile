PROJECT = ant-naming-domains

-include ../../Common.mk

all: release

release: $(BUILD_OUTPUT_DIR)/config/ddclient.conf.mo | $(BUILD_OUTPUT_DIR)
	@echo "built" >> /dev/stderr

$(BUILD_OUTPUT_DIR)/config/ddclient.conf.mo: ./config/ddclient.conf.mo | $(BUILD_OUTPUT_DIR)
	@mkdir -p $(BUILD_OUTPUT_DIR)/config
	@cp ./config/ddclient.conf.mo $(BUILD_OUTPUT_DIR)/config/ddclient.conf.mo

.PHONY: all release
