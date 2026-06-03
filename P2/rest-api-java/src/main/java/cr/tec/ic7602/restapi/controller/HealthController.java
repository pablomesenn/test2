package cr.tec.ic7602.restapi.controller;

import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.time.Instant;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Endpoint propio de salud en /api/health.
 *
 * Spring Boot Actuator ya expone /actuator/health para Kubernetes liveness/readiness,
 * pero este endpoint propio existe para mantener consistencia con el resto del
 * proyecto (la Zonal Cache también expone /health) y para que la Zonal Cache
 * pueda cachearlo y demostrar que NO se cachea (por la regla de la ruta).
 */
@RestController
@RequestMapping("/api")
public class HealthController {

    @GetMapping("/health")
    public Map<String, Object> health() {
        Map<String, Object> body = new LinkedHashMap<>();
        body.put("status", "ok");
        body.put("service", "rest-api-java");
        body.put("timestamp", Instant.now().toString());
        return body;
    }
}
