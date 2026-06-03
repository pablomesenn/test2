package cr.tec.ic7602.restapi.controller;

import cr.tec.ic7602.restapi.dto.ItemRequest;
import cr.tec.ic7602.restapi.dto.ItemResponse;
import cr.tec.ic7602.restapi.service.ItemService;
import jakarta.validation.Valid;
import org.springframework.http.HttpStatus;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.DeleteMapping;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.PutMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.util.UriComponentsBuilder;

import java.net.URI;
import java.util.List;

/**
 * Endpoints REST para Items.
 *
 * Todos producen JSON. La Zonal Cache puede cachearlos según las reglas
 * configuradas en Firestore (ej. extension "json" con TTL específico).
 */
@RestController
@RequestMapping(value = "/api/items", produces = MediaType.APPLICATION_JSON_VALUE)
public class ItemController {

    private final ItemService service;

    public ItemController(ItemService service) {
        this.service = service;
    }

    /** GET /api/items — lista completa. */
    @GetMapping
    public List<ItemResponse> findAll() {
        return service.findAll();
    }

    /** GET /api/items/{id} — uno por id. */
    @GetMapping("/{id}")
    public ItemResponse findById(@PathVariable Long id) {
        return service.findById(id);
    }

    /**
     * POST /api/items — crea un nuevo item.
     *
     * Devuelve 201 Created con el header Location apuntando al recurso creado.
     */
    @PostMapping(consumes = MediaType.APPLICATION_JSON_VALUE)
    public ResponseEntity<ItemResponse> create(
        @Valid @RequestBody ItemRequest body,
        UriComponentsBuilder uriBuilder
    ) {
        ItemResponse created = service.create(body);
        URI location = uriBuilder.path("/api/items/{id}").buildAndExpand(created.getId()).toUri();
        return ResponseEntity.created(location).body(created);
    }

    /** PUT /api/items/{id} — actualiza un item existente. */
    @PutMapping(value = "/{id}", consumes = MediaType.APPLICATION_JSON_VALUE)
    public ItemResponse update(@PathVariable Long id, @Valid @RequestBody ItemRequest body) {
        return service.update(id, body);
    }

    /** DELETE /api/items/{id} — elimina. */
    @DeleteMapping("/{id}")
    @org.springframework.web.bind.annotation.ResponseStatus(HttpStatus.NO_CONTENT)
    public void delete(@PathVariable Long id) {
        service.delete(id);
    }

    /**
     * POST /api/items/seed — inserta items de demo.
     *
     * Pensado para facilitar la revisión virtual: una llamada y hay datos
     * para que la Zonal Cache cachee. Marcado POST porque modifica estado.
     */
    @PostMapping("/seed")
    public List<ItemResponse> seed() {
        return service.seed();
    }
}
