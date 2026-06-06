package cr.tec.ic7602.restapi.repository;

import cr.tec.ic7602.restapi.entity.Item;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

/**
 * Repository CRUD para {@link Item}.
 * Spring Data JPA genera la implementación en runtime.
 */
@Repository
public interface ItemRepository extends JpaRepository<Item, Long> {
    // Operaciones CRUD básicas heredadas de JpaRepository:
    //   findAll(), findById(), save(), deleteById(), count(), existsById()
    // Si más adelante se requieren queries específicas (filtro por precio,
    // búsqueda por nombre), se agregan aquí como métodos derivados o @Query.
}
