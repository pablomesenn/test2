package cr.tec.ic7602.restapi.service;

import cr.tec.ic7602.restapi.dto.ItemRequest;
import cr.tec.ic7602.restapi.dto.ItemResponse;
import cr.tec.ic7602.restapi.entity.Item;
import cr.tec.ic7602.restapi.exception.ResourceNotFoundException;
import cr.tec.ic7602.restapi.repository.ItemRepository;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.math.BigDecimal;
import java.util.List;

/**
 * Capa de servicio para Item.
 *
 * Separa el controller del repository y centraliza la conversión
 * entidad ↔ DTO. Mantenerla delgada es a propósito: este proyecto
 * no requiere lógica de negocio compleja.
 */
@Service
@Transactional
public class ItemService {

    private final ItemRepository repository;

    public ItemService(ItemRepository repository) {
        this.repository = repository;
    }

    /** Lista todos los items, ordenados por id ascendente. */
    @Transactional(readOnly = true)
    public List<ItemResponse> findAll() {
        return repository.findAll().stream()
            .map(ItemResponse::from)
            .toList();
    }

    /** Busca uno por id. Lanza {@link ResourceNotFoundException} si no existe. */
    @Transactional(readOnly = true)
    public ItemResponse findById(Long id) {
        Item item = repository.findById(id)
            .orElseThrow(() -> new ResourceNotFoundException("Item con id " + id + " no encontrado"));
        return ItemResponse.from(item);
    }

    /** Crea un nuevo item a partir del DTO de request. */
    public ItemResponse create(ItemRequest req) {
        Item entity = new Item(req.getName(), req.getDescription(), req.getPrice(), req.getStock());
        return ItemResponse.from(repository.save(entity));
    }

    /** Actualiza un item existente. Lanza 404 si no existe. */
    public ItemResponse update(Long id, ItemRequest req) {
        Item item = repository.findById(id)
            .orElseThrow(() -> new ResourceNotFoundException("Item con id " + id + " no encontrado"));
        item.setName(req.getName());
        item.setDescription(req.getDescription());
        item.setPrice(req.getPrice());
        item.setStock(req.getStock());
        return ItemResponse.from(repository.save(item));
    }

    /** Elimina un item. Lanza 404 si no existe. */
    public void delete(Long id) {
        if (!repository.existsById(id)) {
            throw new ResourceNotFoundException("Item con id " + id + " no encontrado");
        }
        repository.deleteById(id);
    }

    /** Inserta items de muestra. Idempotente respecto al contenido aunque ids varíen. */
    public List<ItemResponse> seed() {
        repository.deleteAll();
        List<Item> samples = List.of(
            new Item("Laptop Lenovo ThinkPad", "Intel i7, 16GB RAM, 512GB SSD",     new BigDecimal("1299.99"), 10),
            new Item("Mouse Logitech MX Master 3", "Mouse inalámbrico ergonómico",  new BigDecimal("99.99"),   25),
            new Item("Teclado Keychron K2", "Teclado mecánico inalámbrico TKL",     new BigDecimal("89.99"),   15),
            new Item("Monitor Dell 27\"", "Monitor 4K USB-C",                       new BigDecimal("449.00"),  8),
            new Item("Webcam Logitech C920", "Webcam Full HD 1080p",                new BigDecimal("79.99"),   30),
            new Item("Headset Sony WH-1000XM5", "Audífonos con cancelación de ruido", new BigDecimal("399.99"), 12),
            new Item("SSD Samsung 980 1TB", "NVMe M.2 PCIe 3.0",                    new BigDecimal("89.99"),   40),
            new Item("Router Wi-Fi 6 ASUS", "Router AX6000",                        new BigDecimal("299.00"),  6),
            new Item("Cable HDMI 2.1", "Cable HDMI 2.1 de 2 metros",                new BigDecimal("19.99"),   100),
            new Item("Hub USB-C", "Hub USB-C 7 en 1 con HDMI",                      new BigDecimal("49.99"),   20)
        );
        return repository.saveAll(samples).stream()
            .map(ItemResponse::from)
            .toList();
    }
}
