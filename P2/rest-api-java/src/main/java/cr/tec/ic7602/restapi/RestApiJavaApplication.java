package cr.tec.ic7602.restapi;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

/**
 * REST API Java - Proyecto 2 IC-7602.
 *
 * Sirve como origen HTTP de prueba para la Zonal Cache:
 * el cache golpea estos endpoints y los almacena según las reglas
 * configuradas en Firestore.
 */
@SpringBootApplication
public class RestApiJavaApplication {

    public static void main(String[] args) {
        SpringApplication.run(RestApiJavaApplication.class, args);
    }
}
