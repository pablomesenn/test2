package cr.tec.ic7602.restapi.controller;

import com.fasterxml.jackson.databind.ObjectMapper;
import cr.tec.ic7602.restapi.dto.ItemRequest;
import cr.tec.ic7602.restapi.dto.ItemResponse;
import cr.tec.ic7602.restapi.exception.ResourceNotFoundException;
import cr.tec.ic7602.restapi.service.ItemService;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.autoconfigure.web.servlet.WebMvcTest;
import org.springframework.boot.test.mock.mockito.MockBean;
import org.springframework.http.MediaType;
import org.springframework.test.web.servlet.MockMvc;

import java.math.BigDecimal;
import java.time.Instant;
import java.util.List;

import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;
import static org.springframework.test.web.servlet.request.MockMvcRequestBuilders.delete;
import static org.springframework.test.web.servlet.request.MockMvcRequestBuilders.get;
import static org.springframework.test.web.servlet.request.MockMvcRequestBuilders.post;
import static org.springframework.test.web.servlet.request.MockMvcRequestBuilders.put;
import static org.springframework.test.web.servlet.result.MockMvcResultMatchers.header;
import static org.springframework.test.web.servlet.result.MockMvcResultMatchers.jsonPath;
import static org.springframework.test.web.servlet.result.MockMvcResultMatchers.status;

/**
 * Tests del controller con MockMvc.
 *
 * Solo carga el contexto web (sin JPA ni service real), aislando
 * los endpoints REST. Para tests de integración end-to-end con base
 * de datos real, ver ItemServiceTest (que sí carga JPA).
 */
@WebMvcTest(ItemController.class)
class ItemControllerTest {

    @Autowired
    private MockMvc mvc;

    @Autowired
    private ObjectMapper objectMapper;

    @MockBean
    private ItemService service;

    private ItemResponse sample(long id, String name) {
        ItemResponse r = new ItemResponse();
        r.setId(id);
        r.setName(name);
        r.setDescription("desc");
        r.setPrice(new BigDecimal("9.99"));
        r.setStock(5);
        r.setCreatedAt(Instant.parse("2026-01-01T00:00:00Z"));
        r.setUpdatedAt(Instant.parse("2026-01-01T00:00:00Z"));
        return r;
    }

    @Test
    @DisplayName("GET /api/items retorna lista")
    void testFindAll() throws Exception {
        when(service.findAll()).thenReturn(List.of(sample(1L, "A"), sample(2L, "B")));
        mvc.perform(get("/api/items"))
            .andExpect(status().isOk())
            .andExpect(jsonPath("$.length()").value(2))
            .andExpect(jsonPath("$[0].name").value("A"))
            .andExpect(jsonPath("$[1].name").value("B"));
    }

    @Test
    @DisplayName("GET /api/items/{id} retorna 200 con el item")
    void testFindById() throws Exception {
        when(service.findById(1L)).thenReturn(sample(1L, "Solo"));
        mvc.perform(get("/api/items/1"))
            .andExpect(status().isOk())
            .andExpect(jsonPath("$.id").value(1))
            .andExpect(jsonPath("$.name").value("Solo"));
    }

    @Test
    @DisplayName("GET /api/items/{id} retorna 404 si no existe")
    void testFindByIdNotFound() throws Exception {
        when(service.findById(99L)).thenThrow(new ResourceNotFoundException("Item con id 99 no encontrado"));
        mvc.perform(get("/api/items/99"))
            .andExpect(status().isNotFound())
            .andExpect(jsonPath("$.status").value(404))
            .andExpect(jsonPath("$.message").value(org.hamcrest.Matchers.containsString("99")));
    }

    @Test
    @DisplayName("POST /api/items con body válido retorna 201 + Location")
    void testCreate() throws Exception {
        when(service.create(any(ItemRequest.class))).thenReturn(sample(42L, "Nuevo"));
        ItemRequest req = new ItemRequest("Nuevo", "desc", new BigDecimal("9.99"), 5);

        mvc.perform(post("/api/items")
                .contentType(MediaType.APPLICATION_JSON)
                .content(objectMapper.writeValueAsString(req)))
            .andExpect(status().isCreated())
            .andExpect(header().string("Location", org.hamcrest.Matchers.endsWith("/api/items/42")))
            .andExpect(jsonPath("$.id").value(42));
    }

    @Test
    @DisplayName("POST /api/items sin name retorna 400 con detalle del campo")
    void testCreateInvalid() throws Exception {
        ItemRequest req = new ItemRequest("", "desc", new BigDecimal("1"), 1);
        mvc.perform(post("/api/items")
                .contentType(MediaType.APPLICATION_JSON)
                .content(objectMapper.writeValueAsString(req)))
            .andExpect(status().isBadRequest())
            .andExpect(jsonPath("$.details.name").exists());
    }

    @Test
    @DisplayName("POST /api/items con precio negativo retorna 400")
    void testCreateNegativePrice() throws Exception {
        ItemRequest req = new ItemRequest("X", "desc", new BigDecimal("-1.00"), 1);
        mvc.perform(post("/api/items")
                .contentType(MediaType.APPLICATION_JSON)
                .content(objectMapper.writeValueAsString(req)))
            .andExpect(status().isBadRequest())
            .andExpect(jsonPath("$.details.price").exists());
    }

    @Test
    @DisplayName("PUT /api/items/{id} actualiza")
    void testUpdate() throws Exception {
        when(service.update(eq(1L), any(ItemRequest.class))).thenReturn(sample(1L, "Updated"));
        ItemRequest req = new ItemRequest("Updated", "x", new BigDecimal("5.00"), 1);

        mvc.perform(put("/api/items/1")
                .contentType(MediaType.APPLICATION_JSON)
                .content(objectMapper.writeValueAsString(req)))
            .andExpect(status().isOk())
            .andExpect(jsonPath("$.name").value("Updated"));
    }

    @Test
    @DisplayName("DELETE /api/items/{id} retorna 204")
    void testDelete() throws Exception {
        mvc.perform(delete("/api/items/1"))
            .andExpect(status().isNoContent());
        verify(service).delete(1L);
    }

    @Test
    @DisplayName("DELETE /api/items/{id} retorna 404 si no existe")
    void testDeleteNotFound() throws Exception {
        doThrow(new ResourceNotFoundException("no")).when(service).delete(99L);
        mvc.perform(delete("/api/items/99"))
            .andExpect(status().isNotFound());
    }

    @Test
    @DisplayName("POST /api/items/seed inserta items de muestra")
    void testSeed() throws Exception {
        when(service.seed()).thenReturn(List.of(sample(1L, "A"), sample(2L, "B")));
        mvc.perform(post("/api/items/seed"))
            .andExpect(status().isOk())
            .andExpect(jsonPath("$.length()").value(2));
    }
}
