events {

}

http {
    upstream fleet {
        server antworker{{ANT_WORKER_NUM_ONE}}.hosts.typesofants.org:{{WEBSERVER_PORT}};
        server antworker{{ANT_WORKER_NUM_TWO}}.hosts.typesofants.org:{{WEBSERVER_PORT}};
        server antworker{{ANT_WORKER_NUM_THREE}}.hosts.typesofants.org:{{WEBSERVER_PORT}};
    } 

    server {
        listen 80;
        server_name {{FQDN}};
        return 301 https://$host$request_uri;
    }

    server {
        listen 443 ssl;
        listen [::]:443 ssl;
        http2 on;
        server_name {{FQDN}};

        location / {
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header Host $http_host;
            proxy_set_header X-NginX-Proxy true;

            proxy_redirect off;
            proxy_pass http://fleet;
        }

        ssl_certificate     /ant-on-the-web/{{FQDN}}/cert.pem;
        ssl_certificate_key /ant-on-the-web/{{FQDN}}/key.pem;
        
        # SSL configuration
        ssl_session_cache shared:le_nginx_SSL:10m;
        ssl_session_timeout 1440m;
        ssl_session_tickets off;

        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_prefer_server_ciphers off;

        ssl_ciphers "ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384";
    }
}