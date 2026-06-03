package cr.tec.ic7602.restapi.exception;

import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.validation.FieldError;
import org.springframework.web.bind.MethodArgumentNotValidException;
import org.springframework.web.bind.annotation.ExceptionHandler;
import org.springframework.web.bind.annotation.RestControllerAdvice;

import java.time.Instant;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.Map;

/**
 * Manejador global de excepciones.
 *
 * Convierte cualquier excepción no capturada del controller en una respuesta
 * JSON con formato consistente. La Zonal Cache ve esta respuesta como cualquier
 * otra, así que el formato uniforme también ayuda a debuggear el cache.
 */
@RestControllerAdvice
public class GlobalExceptionHandler {

    /** 404 — recurso solicitado no existe. */
    @ExceptionHandler(ResourceNotFoundException.class)
    public ResponseEntity<Map<String, Object>> handleNotFound(ResourceNotFoundException ex) {
        return ResponseEntity.status(HttpStatus.NOT_FOUND).body(envelope(
            HttpStatus.NOT_FOUND, ex.getMessage(), null));
    }

    /** 400 — validación de DTO falló (@Valid). */
    @ExceptionHandler(MethodArgumentNotValidException.class)
    public ResponseEntity<Map<String, Object>> handleValidation(MethodArgumentNotValidException ex) {
        Map<String, String> fieldErrors = new HashMap<>();
        for (FieldError fe : ex.getBindingResult().getFieldErrors()) {
            fieldErrors.put(fe.getField(), fe.getDefaultMessage());
        }
        return ResponseEntity.status(HttpStatus.BAD_REQUEST).body(envelope(
            HttpStatus.BAD_REQUEST, "validación fallida", fieldErrors));
    }

    /** 500 — fallback para cualquier otra excepción no esperada. */
    @ExceptionHandler(Exception.class)
    public ResponseEntity<Map<String, Object>> handleGeneric(Exception ex) {
        return ResponseEntity.status(HttpStatus.INTERNAL_SERVER_ERROR).body(envelope(
            HttpStatus.INTERNAL_SERVER_ERROR, "error interno: " + ex.getMessage(), null));
    }

    /** Formato uniforme de error: {timestamp, status, error, message, details?}. */
    private Map<String, Object> envelope(HttpStatus status, String message, Object details) {
        Map<String, Object> body = new LinkedHashMap<>();
        body.put("timestamp", Instant.now().toString());
        body.put("status", status.value());
        body.put("error", status.getReasonPhrase());
        body.put("message", message);
        if (details != null) {
            body.put("details", details);
        }
        return body;
    }
}
