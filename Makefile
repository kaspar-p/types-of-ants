
all: ant-on-the-web ant-data-farm ant-gateway ant-host-agent ant-host-agent-codegen ant-just-checking-in

ant-on-the-web:
	cd projects/ant-on-the-web/server; cargo build;

ant-data-farm:
	docker-compose build ant-data-farm

ant-gateway:
	docker-compose build ant-gateway

ant-host-agent:
	cd projects/ant-host-agent; cargo build;

# ant-host-agent-codegen:
# 	gradlew assemble

ant-just-checking-in:
	cd projects/ant-just-checking-in; cargo build;

PSEUDO: all ant-on-the-web ant-data-farm ant-gateway ant-host-agent ant-host-agent-codegen ant-just-checking-in