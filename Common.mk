BUILD_DIR = build

INSTALL_VERSION = default
INSTALL_PREFIX = $(HOME)/service
SECRETS_DIR = $(HOME)/secrets

BUILD_MARKER := $(TYPESOFANTS_ENV).$(shell uname -s).$(shell uname -m).build-marker

ifeq ($(BUILD_OUTPUT_DIR),)
    $(error BUILD_OUTPUT_DIR is not set. Please define it.)
endif

UNAME_S = $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
# systemd doesn't exist on Mac, put it elsewhere
	SYSTEMD_SERVICES_DIR = $(HOME)/service/systemd/
endif
ifeq ($(UNAME_S),Linux)
	SYSTEMD_SERVICES_DIR = /etc/systemd/system/
endif

$(BUILD_OUTPUT_DIR):
	@mkdir -p $@
