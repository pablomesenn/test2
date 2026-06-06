package cr.tec.ic7602.restapi.dto;

import cr.tec.ic7602.restapi.entity.Item;

import java.math.BigDecimal;
import java.time.Instant;

/**
 * Payload de salida para Item. Mantener esto separado de la entidad evita
 * acoplar la API a cambios internos del modelo de persistencia.
 */
public class ItemResponse {

    private Long id;
    private String name;
    private String description;
    private BigDecimal price;
    private Integer stock;
    private Instant createdAt;
    private Instant updatedAt;

    public ItemResponse() {}

    public static ItemResponse from(Item item) {
        ItemResponse r = new ItemResponse();
        r.id = item.getId();
        r.name = item.getName();
        r.description = item.getDescription();
        r.price = item.getPrice();
        r.stock = item.getStock();
        r.createdAt = item.getCreatedAt();
        r.updatedAt = item.getUpdatedAt();
        return r;
    }

    public Long getId() { return id; }
    public void setId(Long id) { this.id = id; }

    public String getName() { return name; }
    public void setName(String name) { this.name = name; }

    public String getDescription() { return description; }
    public void setDescription(String description) { this.description = description; }

    public BigDecimal getPrice() { return price; }
    public void setPrice(BigDecimal price) { this.price = price; }

    public Integer getStock() { return stock; }
    public void setStock(Integer stock) { this.stock = stock; }

    public Instant getCreatedAt() { return createdAt; }
    public void setCreatedAt(Instant createdAt) { this.createdAt = createdAt; }

    public Instant getUpdatedAt() { return updatedAt; }
    public void setUpdatedAt(Instant updatedAt) { this.updatedAt = updatedAt; }
}
