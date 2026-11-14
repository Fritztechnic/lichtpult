export type Attr = string
export type Kind = 'Intensity' | 'Color' | 'Position' | 'Beam'
export type Values = Record<string, Record<string, number>>
export interface FixtureType { name: string; channels: (Attr | null)[] }
export interface Fixture { id: number; name: string; ty: number; universe: number; address: number; x: number; y: number }
export interface Group { name: string; ids: number[] }
export interface Palette { name: string; kind: Kind; values: Record<string, number> }
export interface Cue { name: string; fade: number; follow: number | null; values: Values; delay: number; comment: string; block: boolean }
export interface Cuelist { name: string; cues: Cue[]; looped: boolean; chase: boolean; bpm: number }
export interface Effect { name: string; target: string; shape: string; rate: number; size: number; spread: number; ids: number[] }
export interface Prop { kind: string; x: number; y: number; w: number; h: number; label: string; color: [number, number, number] | null }
export interface Show {
  name: string; types: FixtureType[]; fixtures: Fixture[]; groups: Group[]; palettes: Palette[]
  lists: Cuelist[]; effects: Effect[]; props: Prop[]
}
export interface Live {
  rev: number; selection: number[]; last_selected: number | null; programmer: Values; parked: Values
  master: number; blackout: boolean; blind: boolean; fade_speed: number; active: number
  can_undo: boolean; can_redo: boolean
  players: { cue: number | null; level: number; progress: number }[]
  fx_running: boolean[]; conflicts: number[]
  fixtures: { id: number; rgb: [number, number, number]; level: number }[]
  frames: Record<string, number[]>
}

export const ATTR_LABEL: Record<string, string> = {
  Dimmer: 'Intensity', Red: 'Red', Green: 'Green', Blue: 'Blue', White: 'White', Amber: 'Amber', Uv: 'UV',
  Pan: 'Pan', Tilt: 'Tilt', PanFine: 'Pan Fine', TiltFine: 'Tilt Fine', Zoom: 'Zoom', Strobe: 'Strobe',
  L1: 'Light 1', L2: 'Light 2', L3: 'Light 3', L4: 'Light 4', L5: 'Light 5', L6: 'Light 6', L7: 'Light 7', L8: 'Light 8',
  ColorWheel: 'Color Wheel', Gobo: 'Gobo', Prism: 'Prism', Iris: 'Iris', Focus: 'Focus', Frost: 'Frost',
}
export const KIND_OF: Record<string, Kind> = {
  Dimmer: 'Intensity', L1: 'Intensity', L2: 'Intensity', L3: 'Intensity', L4: 'Intensity', L5: 'Intensity', L6: 'Intensity', L7: 'Intensity', L8: 'Intensity',
  Red: 'Color', Green: 'Color', Blue: 'Color', White: 'Color', Amber: 'Color', Uv: 'Color', ColorWheel: 'Color',
  Pan: 'Position', Tilt: 'Position', PanFine: 'Position', TiltFine: 'Position',
  Zoom: 'Beam', Strobe: 'Beam', Gobo: 'Beam', Prism: 'Beam', Iris: 'Beam', Focus: 'Beam', Frost: 'Beam',
}
export const ATTR_ORDER = Object.keys(ATTR_LABEL)
export const ATTR_COLOR: Record<string, string> = {
  Red: '#d93030', Green: '#2fb34a', Blue: '#2f6fe0', White: '#e8e8e8', Amber: '#e8a020', Uv: '#8a3fd0',
}
export const SHAPES = ['Sine', 'Triangle', 'Saw', 'Square', 'Cosine', 'RampDown', 'Step', 'Bounce']
export const TARGETS: [string, string][] = [['Intensity', 'Intensity'], ['Color', 'Rainbow'], ['Position', 'Circle']]
export const PROP_KINDS = ['Podest', 'Truss', 'Lautsprecher', 'Pult', 'Publikum', 'Text', 'Vorhang', 'Mikrofon', 'Treppe', 'Bildschirm', 'Rechteck']
export const PROP_LABEL: Record<string, string> = {
  Podest: 'Riser', Truss: 'Truss', Lautsprecher: 'Speaker', Pult: 'Desk', Publikum: 'Audience', Text: 'Label',
  Vorhang: 'Curtain', Mikrofon: 'Microphone', Treppe: 'Stairs', Bildschirm: 'Screen', Rechteck: 'Rectangle',
}
