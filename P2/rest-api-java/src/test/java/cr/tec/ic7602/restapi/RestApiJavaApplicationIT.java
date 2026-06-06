package cr.tec.ic7602.restapi;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import cr.tec.ic7602.restapi.dto.ItemRequest;
import cr.tec.ic7602.restapi.repository.ItemRepository;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.context.SpringBootTest;
import org.springframework.boot.test.web.client.TestRestTemplate;
import org.springframework.http.HttpEntity;
import org.springframework.http.HttpHeaders;
import org.springframework.http.HttpMethod;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;

import java.math.BigDecimal;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * Test de integración: arranca la app completa en un puerto random
 * y le pega con HTTP real. Esto es lo más cercano a "lo que el
 * profe va a hacer en la revisión virtual".
 */
@SpringBootTest(webEnvironment = SpringBootTest.WebEnvironment.RANDOM_PORT)
class RestApiJavaApplicationIT {

    @Autowired
    private TestRestTemplate rest;

    @Autowired
    private ObjectMapper objectMapper;

    @Autowired
    private ItemRepository repository;

    @BeforeEach
    void clean() {
        repository.deleteAll();
    }

    @Test
    @DisplayName("Flujo end-to-end: crear → listar → leer → actualizar → eliminar")
    void testFullCrudFlow() throws Exception {
        // 1) Crear
        ItemRequest createReq = new ItemRequest("E2E Item", "desc", new BigDecimal("10.00"), 3);
        ResponseEntity<String> createRes = rest.postForEntity("/api/items", entity(createReq), String.class);
        assertThat(createRes.getStatusCode().value()).isEqualTo(201);
        assertThat(createRes.getHeaders().getLocation()).isNotNull();
        JsonNode created = objectMapper.readTree(createRes.getBody());
        long id = created.get("id").asLong();

        // 2) Listar — debe tener exactamente uno.
        ResponseEntity<String> listRes = rest.getForEntity("/api/items", String.class);
        assertThat(listRes.getStatusCode().value()).isEqualTo(200);
        JsonNode list = objectMapper.readTree(listRes.getBody());
        assertThat(list.isArray()).isTrue();
        assertThat(list.size()).isEqualTo(1);

        // 3) Leer por id
        ResponseEntity<String> getRes = rest.getForEntity("/api/items/" + id, String.class);
        assertThat(getRes.getStatusCode().value()).isEqualTo(200);
        assertThat(objectMapper.readTree(getRes.getBody()).get("name").asText()).isEqualTo("E2E Item");

        // 4) Actualizar
        ItemRequest updateReq = new ItemRequest("E2E Updated", "desc2", new BigDecimal("20.00"), 5);
        HttpEntity<ItemRequest> putEntity = entity(updateReq);
        ResponseEntity<String> putRes = rest.exchange("/api/items/" + id, HttpMethod.PUT, putEntity, String.class);
        assertThat(putRes.getStatusCode().value()).isEqualTo(200);
        assertThat(objectMapper.readTree(putRes.getBody()).get("name").asText()).isEqualTo("E2E Updated");

        // 5) Eliminar
        ResponseEntity<Void> delRes = rest.exchange("/api/items/" + id, HttpMethod.DELETE, null, Void.class);
        assertThat(delRes.getStatusCode().value()).isEqualTo(204);

        // 6) Verificar que ya no existe
        ResponseEntity<String> notFound = rest.getForEntity("/api/items/" + id, String.class);
        assertThat(notFound.getStatusCode().value()).isEqualTo(404);
    }

    @Test
    @DisplayName("Validación 400 viaja end-to-end con detalles por campo")
    void testValidationEndToEnd() throws Exception {
        ItemRequest bad = new ItemRequest("", null, new BigDecimal("-1"), -5);
        ResponseEntity<String> res = rest.postForEntity("/api/items", entity(bad), String.class);
        assertThat(res.getStatusCode().value()).isEqualTo(400);
        JsonNode body = objectMapper.readTree(res.getBody());
        assertThat(body.get("status").asInt()).isEqualTo(400);
        assertThat(body.get("details").has("name")).isTrue();
        assertThat(body.get("details").has("price")).isTrue();
        assertThat(body.get("details").has("stock")).isTrue();
    }

    @Test
    @DisplayName("POST /api/items/seed inserta 10 items reproducibles")
    void testSeedEndToEnd() throws Exception {
        ResponseEntity<String> res = rest.postForEntity("/api/items/seed", null, String.class);
        assertThat(res.getStatusCode().value()).isEqualTo(200);
        JsonNode body = objectMapper.readTree(res.getBody());
        assertThat(body.isArray()).isTrue();
        assertThat(body.size()).isEqualTo(10);
        assertThat(repository.count()).isEqualTo(10);
    }

    @Test
    @DisplayName("GET /api/health responde 200 ok")
    void testHealth() throws Exception {
        ResponseEntity<String> res = rest.getForEntity("/api/health", String.class);
        assertThat(res.getStatusCode().value()).isEqualTo(200);
        assertThat(objectMapper.readTree(res.getBody()).get("status").asText()).isEqualTo("ok");
    }

    private HttpEntity<ItemRequest> entity(ItemRequest req) {
        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        return new HttpEntity<>(req, headers);
    }
}
