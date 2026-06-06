package cr.tec.ic7602.restapi.exception;

/**
 * Excepción lanzada cuando un recurso solicitado no existe.
 * El {@code GlobalExceptionHandler} la mapea a HTTP 404.
 */
public class ResourceNotFoundException extends RuntimeException {
    public ResourceNotFoundException(String message) {
        super(message);
    }
}
