PROJECT = ant-naming-domains

-include ../../Makefile.common

all: release

release:
	mkdir -p ./build/release/config
	ANT_NAMING_DOMAINS_FQDN=$(ANT_NAMING_DOMAINS_FQDN) mo ./config/ddclient.conf.mo > ./build/release/config/ddclient.conf

.PHONY: all release
