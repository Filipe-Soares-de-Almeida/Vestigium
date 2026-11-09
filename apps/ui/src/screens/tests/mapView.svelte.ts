// Whether disk maps show each region's response time, remembered by this window between openings.
const KEY = "vestigium.map.speed";

function recall(): boolean {
  try {
    return localStorage.getItem(KEY) === "1";
  } catch {
    return false;
  }
}

class MapView {
  speed = $state(recall());

  setSpeed(on: boolean): void {
    this.speed = on;
    try {
      localStorage.setItem(KEY, on ? "1" : "0");
    } catch {
      // Without storage the choice lasts until the window closes.
    }
  }
}

export const mapView = new MapView();
