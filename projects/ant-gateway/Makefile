PROJECT = ant-gateway

-include ../../Makefile.common

all:
	docker-compose build ant-gateway

release: all
install: all

.PHONY: all release install
