package cr.tec.ic7602.restapi.service;

import cr.tec.ic7602.restapi.dto.ItemRequest;
import cr.tec.ic7602.restapi.dto.ItemResponse;
import cr.tec.ic7602.restapi.exception.ResourceNotFoundException;
import cr.tec.ic7602.restapi.repository.ItemRepository;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.test.autoconfigure.orm.jpa.DataJpaTest;
import org.springframework.context.annotation.Import;

import java.math.BigDecimal;
import java.util.List;

import static org.assertj.core.api.Assertions.assertThat;
import static org.assertj.core.api.Assertions.assertThatThrownBy;

/**
 * Tests del servicio con H2 real (via @DataJpaTest).
 *
 * Probar contra una base de datos real (aunque sea en memoria) detecta
 * bugs de mapeo JPA que un mock nunca encontraría.
 */
@DataJpaTest
@Import(ItemService.class)
class ItemServiceTest {

    @Autowired
    private ItemService service;

    @Autowired
    private ItemRepository repository;

    @BeforeEach
    void cleanDatabase() {
        repository.deleteAll();
    }

    @Test
    @DisplayName("create persiste con timestamps y retorna ItemResponse")
    void testCreate() {
        ItemRequest req = new ItemRequest("Mouse", "Logitech MX", new BigDecimal("99.99"), 10);
        ItemResponse r = service.create(req);

        assertThat(r.getId()).isNotNull();
        assertThat(r.getName()).isEqualTo("Mouse");
        assertThat(r.getPrice()).isEqualByComparingTo("99.99");
        assertThat(r.getCreatedAt()).isNotNull();
        assertThat(r.getUpdatedAt()).isNotNull();
    }

    @Test
    @DisplayName("findAll retorna todos los items persistidos")
    void testFindAll() {
        service.create(new ItemRequest("A", "desc", new BigDecimal("1.00"), 1));
        service.create(new ItemRequest("B", "desc", new BigDecimal("2.00"), 2));

        List<ItemResponse> all = service.findAll();
        assertThat(all).hasSize(2);
        assertThat(all).extracting(ItemResponse::getName).containsExactlyInAnyOrder("A", "B");
    }

    @Test
    @DisplayName("findById retorna el item correcto")
    void testFindById() {
        ItemResponse created = service.create(new ItemRequest("Solo", "x", new BigDecimal("5.00"), 1));
        ItemResponse fetched = service.findById(created.getId());
        assertThat(fetched.getId()).isEqualTo(created.getId());
        assertThat(fetched.getName()).isEqualTo("Solo");
    }

    @Test
    @DisplayName("findById lanza ResourceNotFoundException si no existe")
    void testFindByIdNotFound() {
        assertThatThrownBy(() -> service.findById(99999L))
            .isInstanceOf(ResourceNotFoundException.class)
            .hasMessageContaining("99999");
    }

    @Test
    @DisplayName("update modifica los campos del item")
    void testUpdate() {
        ItemResponse created = service.create(new ItemRequest("Old", "desc", new BigDecimal("1.00"), 1));

        ItemRequest updateReq = new ItemRequest("New", "desc2", new BigDecimal("2.00"), 5);
        ItemResponse updated = service.update(created.getId(), updateReq);

        assertThat(updated.getId()).isEqualTo(created.getId());
        assertThat(updated.getName()).isEqualTo("New");
        assertThat(updated.getPrice()).isEqualByComparingTo("2.00");
        assertThat(updated.getStock()).isEqualTo(5);
        assertThat(updated.getUpdatedAt()).isAfterOrEqualTo(created.getUpdatedAt());
    }

    @Test
    @DisplayName("update sobre id inexistente lanza 404")
    void testUpdateNotFound() {
        ItemRequest req = new ItemRequest("X", "Y", new BigDecimal("1.00"), 1);
        assertThatThrownBy(() -> service.update(99999L, req))
            .isInstanceOf(ResourceNotFoundException.class);
    }

    @Test
    @DisplayName("delete elimina el item del repositorio")
    void testDelete() {
        ItemResponse created = service.create(new ItemRequest("A", "x", new BigDecimal("1.00"), 1));
        service.delete(created.getId());
        assertThat(repository.existsById(created.getId())).isFalse();
    }

    @Test
    @DisplayName("delete sobre id inexistente lanza 404")
    void testDeleteNotFound() {
        assertThatThrownBy(() -> service.delete(99999L))
            .isInstanceOf(ResourceNotFoundException.class);
    }

    @Test
    @DisplayName("seed inserta exactamente 10 items y elimina los anteriores")
    void testSeed() {
        service.create(new ItemRequest("Pre", "x", new BigDecimal("1.00"), 1));
        List<ItemResponse> seeded = service.seed();
        assertThat(seeded).hasSize(10);
        assertThat(repository.count()).isEqualTo(10);
        // El item pre-existente debe haber sido borrado.
        assertThat(seeded).extracting(ItemResponse::getName).doesNotContain("Pre");
    }
}
