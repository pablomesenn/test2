package cr.tec.ic7602.restapi.config;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.web.servlet.config.annotation.CorsRegistry;
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer;

/**
 * Configuración CORS.
 *
 * La API Java es consumida principalmente por la Zonal Cache (server-to-server),
 * pero también puede llamarse directamente desde la UI durante desarrollo o demos.
 *
 * Los orígenes permitidos vienen de la variable de entorno CORS_ALLOWED_ORIGINS
 * (lista separada por comas). En producción, configurar solo los orígenes reales;
 * en desarrollo, "*" funciona pero deshabilita el envío de credenciales.
 */
@Configuration
public class CorsConfig {

    @Value("${app.cors.allowed-origins:*}")
    private String[] allowedOrigins;

    @Bean
    public WebMvcConfigurer corsConfigurer() {
        return new WebMvcConfigurer() {
            @Override
            public void addCorsMappings(CorsRegistry registry) {
                registry.addMapping("/api/**")
                    .allowedOrigins(allowedOrigins)
                    .allowedMethods("GET", "POST", "PUT", "DELETE", "OPTIONS")
                    .allowedHeaders("*")
                    .maxAge(3600);
            }
        };
    }
}
