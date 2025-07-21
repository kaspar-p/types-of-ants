PROJECT = ant-renewing-certificates

all: release

release:
	mkdir -p build/release/getssl
	cp -r ./getssl/. build/release/getssl/
	mkdir -p build/release/getssl/$(ANT_RENEWING_CERTIFICATES_FQDN)

	touch build/release/getssl/$(ANT_RENEWING_CERTIFICATES_FQDN)/FORCE_RENEWAL
	cp -r ./$(ANT_RENEWING_CERTIFICATES_FQDN)/getssl.cfg build/release/getssl/$(ANT_RENEWING_CERTIFICATES_FQDN)

.PHONY: release
