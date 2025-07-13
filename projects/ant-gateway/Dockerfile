FROM nginx

# Use NGINX interpolation to turn this file into /etc/nginx/conf.d/default.conf,
# overwriting the previous file there.
COPY nginx.conf /etc/nginx/templates/default.conf.template

CMD ["nginx", "-g", "daemon off;"]
