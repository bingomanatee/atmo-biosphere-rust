
# Simulation Starting Scenario

## 1. Magma Ocean Initialization
- **Region**: Top 100 km of the planet’s shell  
- **State**: Fully molten liquid silicate  
- **Properties**:  
  - High heat capacity  
  - Latent heat of crystallization  
  - Low viscosity  
- **Initial Temperature**: 1,473 K (basaltic solidus)  

## 2. Radiative Cooling at the Surface
- **Mechanism**: Outgoing infrared radiation to space  
- **Calculation**:  
  ```
  heatLost = emissivity × StefanBoltzmannConstant × (surfaceTemperature⁴) × area
  ```
- **Emissivity**: ~0.9 default  
- **Effect**: Reduces energy in the topmost layer each timestep  

## 3. Phase Change & Crust Formation
- **Trigger**: Layer temperature falls below 1,473 K  
- **Process**:
  1. Remove latent heat of crystallization (≈400 kJ/kg)  
  2. Change material properties to **solid silicate** (crust)  
  3. Define this as a 5 km crustal slab  

## 4. Vertical Heat Conduction
- **Between Layers**:
  ```
  heatFlux = thermalConductivity × (T_upper – T_lower) / layerThickness
  ```
- **Update**: Adjust temperatures of both layers based on heatFlux  

## 5. Mantle Heat Input
- **Representation**: Constant upward heat flux at the bottom (~0.07 W/m²)  
- **Purpose**: Models convective upwelling energy into the base of the column  

## 6. Layer-by-Layer Growth
- **Crust Layers**: 5 km thick, appended as each top layer solidifies  
- **Mantle Layers**: 50 km thick slices below, initially molten/ductile until cooled below ~1,600 K  

## 7. Volatile Outgassing & Ocean Formation
- **During Molten Phase**: Release H₂O, CO₂, and other gases into atmosphere  
- **Condensation**: Once surface temperature < 647 K (water critical point), form liquid water layers  

## 8. Transition to Tectonic Model
- **Condition**: Rigid lid thickness reaches ~10–20 km  
- **Next Steps**:
  - Activate plate motions  
  - Model asthenospheric convection  
  - Simulate decompression melting at upwelling zones  
  - Include episodic pluton emplacement and continental crust growth  
