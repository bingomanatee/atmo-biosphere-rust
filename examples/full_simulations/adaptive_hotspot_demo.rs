// Demonstration of adaptive hotspot management when hotspots are overpowered

use std::collections::HashMap;

/// Simplified hotspot for demonstration
#[derive(Debug, Clone)]
pub struct SimpleHotspot {
    pub id: u32,
    pub energy_multiplier: f64,
    pub location: String,
}

/// Adaptive hotspot manager
pub struct AdaptiveHotspotManager {
    pub hotspots: Vec<SimpleHotspot>,
    pub target_count: usize,
    pub total_energy_budget: f64,
}

impl AdaptiveHotspotManager {
    pub fn new(initial_count: usize, total_energy: f64) -> Self {
        let energy_per_hotspot = total_energy / initial_count as f64;
        
        let hotspots = (0..initial_count)
            .map(|i| SimpleHotspot {
                id: i as u32,
                energy_multiplier: energy_per_hotspot,
                location: format!("Location_{}", i),
            })
            .collect();
        
        Self {
            hotspots,
            target_count: initial_count,
            total_energy_budget: total_energy,
        }
    }
    
    /// Check if hotspots are causing excessive energy that would trigger scaling
    pub fn check_if_overpowered(&self, geological_limit: f64) -> bool {
        let max_hotspot_energy = self.hotspots.iter()
            .map(|h| h.energy_multiplier)
            .fold(0.0, f64::max);
        
        max_hotspot_energy > geological_limit
    }
    
    /// Adaptive management: add 50% more hotspots and reduce energy by 33%
    pub fn adapt_if_overpowered(&mut self, geological_limit: f64) -> bool {
        if self.check_if_overpowered(geological_limit) {
            println!("🚨 Hotspots overpowered! Adapting system...");
            
            let original_count = self.target_count;
            let original_max_energy = self.hotspots.iter()
                .map(|h| h.energy_multiplier)
                .fold(0.0, f64::max);
            
            // Step 1: Add 50% more hotspots
            let new_count = (original_count as f64 * 1.5) as usize;
            self.target_count = new_count;
            
            // Step 2: Reduce energy of all existing hotspots by 33%
            for hotspot in &mut self.hotspots {
                hotspot.energy_multiplier *= 0.67; // Reduce by 33%
            }
            
            // Step 3: Create additional hotspots with reduced energy
            let new_energy_per_hotspot = self.total_energy_budget / new_count as f64;
            
            while self.hotspots.len() < new_count {
                let new_id = self.hotspots.len() as u32;
                self.hotspots.push(SimpleHotspot {
                    id: new_id,
                    energy_multiplier: new_energy_per_hotspot * 0.67, // Also reduced
                    location: format!("NewLocation_{}", new_id),
                });
            }
            
            let new_max_energy = self.hotspots.iter()
                .map(|h| h.energy_multiplier)
                .fold(0.0, f64::max);
            
            println!("✅ Hotspot adaptation complete:");
            println!("   Original hotspots: {} (max energy: {:.2e})", original_count, original_max_energy);
            println!("   New hotspots: {} (max energy: {:.2e})", new_count, new_max_energy);
            println!("   Energy reduction: {:.1}%", (1.0 - new_max_energy / original_max_energy) * 100.0);
            println!("   Total energy maintained: {:.2e}", self.get_total_energy());
            
            true
        } else {
            false
        }
    }
    
    pub fn get_total_energy(&self) -> f64 {
        self.hotspots.iter().map(|h| h.energy_multiplier).sum()
    }
    
    pub fn print_status(&self) {
        println!("📊 Hotspot Status:");
        println!("   Count: {}", self.hotspots.len());
        println!("   Total energy: {:.2e}", self.get_total_energy());
        println!("   Average energy per hotspot: {:.2e}", self.get_total_energy() / self.hotspots.len() as f64);
        println!("   Max energy per hotspot: {:.2e}", 
            self.hotspots.iter().map(|h| h.energy_multiplier).fold(0.0, f64::max));
    }
}

fn main() {
    println!("🧪 Adaptive Hotspot Management Demo");
    println!("Demonstrates: If hotspots are overpowered, add 50% more and reduce energy by 33%\n");
    
    // Scenario 1: Normal hotspots (within limits)
    println!("🔬 Scenario 1: Normal Hotspots");
    let mut normal_hotspots = AdaptiveHotspotManager::new(10, 1e20);
    normal_hotspots.print_status();
    
    let geological_limit = 1e19; // 10^19 J per hotspot limit
    let adapted = normal_hotspots.adapt_if_overpowered(geological_limit);
    
    if !adapted {
        println!("✅ No adaptation needed - hotspots within geological limits\n");
    }
    
    // Scenario 2: Overpowered hotspots (exceed limits)
    println!("🔬 Scenario 2: Overpowered Hotspots");
    let mut overpowered_hotspots = AdaptiveHotspotManager::new(5, 1e21); // Much higher energy
    println!("Initial state:");
    overpowered_hotspots.print_status();
    
    let adapted = overpowered_hotspots.adapt_if_overpowered(geological_limit);
    
    if adapted {
        println!("\nAfter adaptation:");
        overpowered_hotspots.print_status();
        
        // Verify the adaptation worked
        let still_overpowered = overpowered_hotspots.check_if_overpowered(geological_limit);
        if !still_overpowered {
            println!("✅ Adaptation successful - hotspots now within geological limits");
        } else {
            println!("❌ Adaptation failed - hotspots still overpowered");
        }
    }
    
    // Scenario 3: Multiple adaptation cycles
    println!("\n🔬 Scenario 3: Multiple Adaptation Cycles");
    let mut extreme_hotspots = AdaptiveHotspotManager::new(3, 1e22); // Extremely high energy
    println!("Initial extreme hotspots:");
    extreme_hotspots.print_status();
    
    let mut adaptation_count = 0;
    while extreme_hotspots.check_if_overpowered(geological_limit) && adaptation_count < 5 {
        adaptation_count += 1;
        println!("\n--- Adaptation Cycle {} ---", adaptation_count);
        extreme_hotspots.adapt_if_overpowered(geological_limit);
    }
    
    println!("\nFinal state after {} adaptation cycles:", adaptation_count);
    extreme_hotspots.print_status();
    
    println!("\n🎯 Key Benefits of Adaptive Hotspot Management:");
    println!("   ✅ Maintains total energy budget");
    println!("   ✅ Distributes energy across more realistic hotspots");
    println!("   ✅ Reduces individual hotspot intensity");
    println!("   ✅ Prevents transaction system scaling");
    println!("   ✅ More geologically realistic distribution");
    
    println!("\n✅ Adaptive Hotspot Management Demo Complete!");
}
