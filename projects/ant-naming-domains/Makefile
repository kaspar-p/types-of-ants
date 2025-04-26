PROJECT = ant-gateway

-include ../../Makefile.common

all:
	docker-compose build ant-naming-domains

release: all
install: all

.PHONY: all release install
