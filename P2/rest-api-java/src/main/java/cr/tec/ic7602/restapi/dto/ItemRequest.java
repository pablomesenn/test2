package cr.tec.ic7602.restapi.dto;

import jakarta.validation.constraints.DecimalMin;
import jakarta.validation.constraints.Min;
import jakarta.validation.constraints.NotBlank;
import jakarta.validation.constraints.NotNull;
import jakarta.validation.constraints.Size;

import java.math.BigDecimal;

/**
 * Payload de entrada para crear o actualizar un Item.
 * La validación se dispara automáticamente con {@code @Valid} en el controller.
 */
public class ItemRequest {

    @NotBlank(message = "el nombre es requerido")
    @Size(max = 200, message = "el nombre no puede exceder 200 caracteres")
    private String name;

    @Size(max = 1000, message = "la descripción no puede exceder 1000 caracteres")
    private String description;

    @NotNull(message = "el precio es requerido")
    @DecimalMin(value = "0.0", inclusive = true, message = "el precio no puede ser negativo")
    private BigDecimal price;

    @NotNull(message = "el stock es requerido")
    @Min(value = 0, message = "el stock no puede ser negativo")
    private Integer stock;

    public ItemRequest() {}

    public ItemRequest(String name, String description, BigDecimal price, Integer stock) {
        this.name = name;
        this.description = description;
        this.price = price;
        this.stock = stock;
    }

    public String getName() { return name; }
    public void setName(String name) { this.name = name; }

    public String getDescription() { return description; }
    public void setDescription(String description) { this.description = description; }

    public BigDecimal getPrice() { return price; }
    public void setPrice(BigDecimal price) { this.price = price; }

    public Integer getStock() { return stock; }
    public void setStock(Integer stock) { this.stock = stock; }
}
