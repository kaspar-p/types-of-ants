PROJECT = ant-renewing-certificates

all: release

release:
	mkdir -p build/release
	ANT_RENEWING_CERTIFICATES_FQDN=$(ANT_RENEWING_CERTIFICATES_FQDN) mo cronjobs.mo > build/release/cronjobs

.PHONY: release
