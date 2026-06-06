# Imagen base oficial Alpine — mínima y segura
FROM httpd:2.4-alpine

# Metadatos
LABEL maintainer="Equipo P2 IC-7602"
LABEL description="Apache HTTP Server — origen estático para Zonal Cache IC-7602 P2"

# Variables de entorno con defaults (sobreescribibles via Helm/Docker)
ENV SERVER_NAME=apache-server.local
ENV SERVER_PORT=80

# Copiar la página HTML al directorio de documentos de Apache
COPY html/ /usr/local/apache2/htdocs/

# Copiar configuración personalizada
COPY httpd.conf /usr/local/apache2/conf/httpd.conf

# Exponer el puerto HTTP
EXPOSE 80

# Apache ya tiene su propio CMD en la imagen base (httpd-foreground)