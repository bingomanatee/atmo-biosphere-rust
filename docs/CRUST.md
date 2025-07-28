
```js
// Constants
const gravity = 9.81;            // m/s²
const gasConstant = 8.314;       // J/(mol·K)

// Material parameters (example values)
const crust = {
surfaceDensity: 2800,          // kg/m³
bulkModulus: 5e10,             // Pa
thermalConductivity: 3,        // W/(m·K)
baseViscosity: 1e21,           // Pa·s
activationEnergy: 2e5,         // J/mol
activationVolume: 1e-5         // m³/mol
};

// Given for each layer:
function computeLayerProperties(layers, hexArea) {
let massAbove = 0;
return layers.map(layer => {
// 1. Overburden pressure
const overburdenPressure = (massAbove * gravity) / hexArea;

    // 2. Compressed density
    const density = layer.surfaceDensity 
                    * (1 + overburdenPressure / layer.bulkModulus);
    
    // 3. Layer mass (for next iteration)
    const layerVolume = layer.thickness * hexArea;
    const layerMass = density * layerVolume;
    massAbove += layerMass;
    
    // 4. Conductive heat flux (if we know temperature gradient)
    const heatFlux = - layer.thermalConductivity 
                     * layer.temperatureGradient;
    
    // 5. Convective viscosity
    const flowViscosity = layer.baseViscosity 
      * Math.exp(
          (layer.activationEnergy 
            + overburdenPressure * layer.activationVolume)
          / (gasConstant * layer.absoluteTemperature)
        );
    
    return {
      density,
      overburdenPressure,
      layerMass,
      heatFlux,
      flowViscosity
    };
});
}

// Example layer definitions
const layers = [
{
// Crust layer
surfaceDensity: crust.surfaceDensity,
bulkModulus: crust.bulkModulus,
thermalConductivity: crust.thermalConductivity,
baseViscosity: crust.baseViscosity,
activationEnergy: crust.activationEnergy,
activationVolume: crust.activationVolume,
thickness: 5000,                // 5 km in meters
temperatureGradient: 0.025,     // K/m (25 K per km)
absoluteTemperature: 300 + 273  // K (e.g., 300 °C)
},
// Add mantle layers similarly...
];

const hexArea = /* compute using H3 edge length */;
const results = computeLayerProperties(layers, hexArea);
console.log(results);



```
Compression is 
```rust
// after loading your JSON:
const surfaceDensity = mat.solid.density_kg_m3;
const bulkModulus    = mat.solid.bulk_modulus_pa;
const pressure       = massAbove * gravity / hexArea;

// “good‐enough” density under pressure:
const compressedDensity = surfaceDensity * (1 + pressure / bulkModulus);

```