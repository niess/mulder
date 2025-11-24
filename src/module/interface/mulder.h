#ifndef mulder_h
#define mulder_h
#ifdef __cplusplus
extern "C" {
#endif

/* C standard library. */
#include <stddef.h>


/* ============================================================================
 *  C module interface.
 * ============================================================================
 */
struct mulder_module {
    /* Returns an element definition */
    struct mulder_element * (*element)(const char * symbol);

    /* Returns a geometry definition. */
    struct mulder_geometry * (*geometry)(void);

    /* Returns a material definition */
    struct mulder_material * (*material)(const char * name);
};

struct mulder_module mulder_initialise(void);


/* ============================================================================
 *  Geometry definition interface.
 * ============================================================================
 */
struct mulder_geometry {
    /* Destroys the geometry definition. */
    void (*destroy)(struct mulder_geometry * self);

    /* Returns the total number of media composing this geometry. */
    size_t (*media_len)(const struct mulder_geometry * self);

    /* Returns data relative to a specific medium. */
    struct mulder_medium * (*medium)(
        const struct mulder_geometry * self,
        size_t index
    );

    /* Returns a tracer for this geometry. */
    struct mulder_tracer * (*tracer)(const struct mulder_geometry * geometry);
};


/* ============================================================================
 *  Geometry tracer interface.
 * ============================================================================
 */
struct mulder_vec3 {
    double x, y, z;
};

struct mulder_tracer {
    /* Destroys the geometry tracer. */
    void (*destroy)(struct mulder_tracer * self);

    // Locates the medium at the given position. //
    size_t (*locate)(
        struct mulder_tracer * self,
        struct mulder_vec3 position
    );

    // Resets the tracer for a new run. //
    void (*reset)(
        struct mulder_tracer * self,
        struct mulder_vec3 position,
        struct mulder_vec3 direction
    );

    // Performs a tracing step. //
    double (*trace)(
        struct mulder_tracer * self,
        double max_length
    );

    // Updates the tracer position.
    void (*move)(
        struct mulder_tracer * self,
        double length
    );

    // Updates the tracer direction.
    void (*turn)(
        struct mulder_tracer * self,
        struct mulder_vec3 direction
    );

    // Returns the current medium. //
    size_t (*medium)(struct mulder_tracer * self);

    // Returns the current position. //
    struct mulder_vec3 (*position)(struct mulder_tracer * self);
};


/* ============================================================================
 *  Medium interface.
 * ============================================================================
 */
struct mulder_medium {
    /* Destroys the medium definition. */
    void (*destroy)(struct mulder_medium * self);

    /* Returns the name of the constitutive material. */
    const char * (*material)(const struct mulder_medium * self);

    /* Optionaly, returns the bulk density of this medium, in kg/m3. */
    double (*density)(const struct mulder_medium * self);

    /* Optionaly, returns a brief description of this geometry medium. */
    const char * (*description)(const struct mulder_medium * self);
};


/* ============================================================================
 *  Material definition interface.
 * ============================================================================
 */
struct mulder_material {
    /* Destroys the material definition. */
    void (*destroy)(struct mulder_material * self);

    /* Returns a specific component. */
    struct mulder_component * (*component)(
        const struct mulder_material * self,
        size_t index
    );

    /* Returns the number of components. */
    size_t (*components_len)(const struct mulder_material * self);

    /* Returns the material density, in kg/m3. */
    double (*density)(const struct mulder_material * self);

    /* Optionaly, returns the material Mean Excitation Energy, in GeV. */
    double (*I)(const struct mulder_material * self);
};

struct mulder_component {
    /* Destroys the component. */
    void (*destroy)(struct mulder_component * self);

    /* Returns the element symbol. */
    const char * (*symbol)(const struct mulder_component * self);

    /* Returns the molar weight of this element. */
    double (*weight)(const struct mulder_component * self);
};

struct mulder_element {
    /* Destroys the element definition. */
    void (*destroy)(struct mulder_element * self);

    /* Returns the mass number of this element. */
    double (*A)(const struct mulder_element * self);

    /* Returns the Mean Excitation Energy, in GeV. */
    double (*I)(const struct mulder_element * self);

    /* Returns the atomic number of this element. */
    int (*Z)(const struct mulder_element * self);
};

#ifdef __cplusplus
}
#endif
#endif
