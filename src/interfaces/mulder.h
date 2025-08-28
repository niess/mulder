#ifndef mulder_h
#define mulder_h
#ifdef __cplusplus
extern "C" {
#endif

/* C standard library. */
#include <stddef.h>


/* ============================================================================
 *  External geometry interface.
 * ============================================================================
 */
struct mulder_interface {
    struct mulder_geometry_definition * (*definition)(void);

    struct mulder_geometry_tracer * (*tracer)(
        const struct mulder_geometry_definition * definition
    );
};

struct mulder_interface mulder_initialise(void);


/* ============================================================================
 *  Geometry definition interface.
 * ============================================================================
 */
struct mulder_geometry_definition {
    /* Destroys the geometry definition. */
    void (*destroy)(struct mulder_geometry_definition * self);

    /* Returns the definition of a constitutive material. */
    struct mulder_material_definition * (*material)(
        const struct mulder_geometry_definition * self,
        size_t index
    );

    /* Returns the total number of materials for this geometry. */
    size_t (*materials_len)(const struct mulder_geometry_definition * self);

    /* Returns data relative to a specific geometry medium. */
    struct mulder_geometry_medium * (*medium)(
        const struct mulder_geometry_definition * self,
        size_t index
    );

    /* Returns the total number of media composing this geometry. */
    size_t (*media_len)(const struct mulder_geometry_definition * self);
};


/* ============================================================================
 *  Geometry tracer interface.
 * ============================================================================
 */
struct mulder_vec3 {
    double x, y, z;
};

struct mulder_geometry_tracer {
    /* Destroys the geometry tracer. */
    void (*destroy)(struct mulder_geometry_tracer * self);

    // Locates the medium at the given position. //
    size_t (*locate)(
        struct mulder_geometry_tracer * self,
        struct mulder_vec3 position
    );

    // Resets the tracer for a new run. //
    void (*reset)(
        struct mulder_geometry_tracer * self,
        struct mulder_vec3 position,
        struct mulder_vec3 direction
    );

    // Performs a tracing step. //
    double (*trace)(
        struct mulder_geometry_tracer * self,
        double max_length
    );

    // Updates the tracer position.
    void (*update)(
        struct mulder_geometry_tracer * self,
        double length,
        struct mulder_vec3 direction
    );

    // Returns the current medium. //
    size_t (*medium)(struct mulder_geometry_tracer * self);

    // Returns the current position. //
    struct mulder_vec3 (*position)(struct mulder_geometry_tracer * self);
};


/* ============================================================================
 *  Geometry medium interface.
 * ============================================================================
 */
struct mulder_geometry_medium {
    /* Destroys the medium definition. */
    void (*destroy)(struct mulder_geometry_medium * self);

    /* Returns the name of the constitutive material. */
    const char * (*material)(const struct mulder_geometry_medium * self);

    /* Returns the bulk density of this geometry medium, in kg/m3. */
    double (*density)(const struct mulder_material_definition * self);

    /* Returns a brief description of this geometry medium. */
    const char * (*description)(const struct mulder_geometry_medium * self);
};


/* ============================================================================
 *  Material definition interface.
 * ============================================================================
 */
struct mulder_material_definition {
    /* Destroys the material definition. */
    void (*destroy)(struct mulder_material_definition * self);

    /* Returns the material name. */
    const char * (*name)(const struct mulder_material_definition * self);

    /* Optionaly, returns the material density, in kg/m3. */
    double (*density)(const struct mulder_material_definition * self);

    /* Optionaly, returns data relative to a specific atomic element. */
    struct mulder_weighted_element * (*element)(
        const struct mulder_material_definition * self,
        size_t index
    );

    /* Optionaly, returns the number of atomic elements. */
    size_t (*elements_len)(const struct mulder_material_definition * self);

    /* Optionaly, returns the material Mean Excitation Energy, in GeV. */
    double (*I)(const struct mulder_material_definition * self);
};

struct mulder_weighted_element {
    /* Destroys the element definition. */
    void (*destroy)(struct mulder_weighted_element * self);

    /* Returns the element symbol. */
    const char * (*symbol)(const struct mulder_weighted_element * self);

    /* Returns the molar weight of this element. */
    double (*weight)(const struct mulder_weighted_element * self);

    /* Optionaly, returns the mass number of this element. */
    double (*A)(const struct mulder_weighted_element * self);

    /* Optionaly, returns the Mean Excitation Energy, in GeV. */
    double (*I)(const struct mulder_weighted_element * self);

    /* Optionaly, returns the atomic number of this element. */
    int (*Z)(const struct mulder_weighted_element * self);
};


#ifdef __cplusplus
}
#endif
#endif
