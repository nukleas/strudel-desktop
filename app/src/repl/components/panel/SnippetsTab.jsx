import React, { useState } from 'react';
import { useSettings } from '@src/settings.mjs';
import {
  MusicalNoteIcon,
  SpeakerWaveIcon,
  SpeakerXMarkIcon,
  MusicalNoteIcon as NoteIcon,
  SpeakerWaveIcon as SynthIcon,
  CircleStackIcon,
  SparklesIcon,
  ArrowPathIcon,
  BoltIcon,
  DocumentTextIcon,
  WrenchScrewdriverIcon,
  MusicalNoteIcon as ScaleIcon,
  UserIcon,
  MusicalNoteIcon as MelodyIcon,
  MusicalNoteIcon as CompleteIcon,
  StarIcon,
  ComputerDesktopIcon,
  MusicalNoteIcon as JazzIcon,
  SpeakerWaveIcon as ElectronicIcon,
  GlobeAltIcon,
} from '@heroicons/react/16/solid';

// Snippet categories inspired by strudel-desktop
const snippetCategories = {
  drums: { name: 'Drums & Beats', icon: MusicalNoteIcon },
  percussion: { name: 'Percussion', icon: SpeakerWaveIcon },
  melodic: { name: 'Melodic & Keys', icon: NoteIcon },
  bass: { name: 'Bass', icon: SpeakerWaveIcon },
  synth: { name: 'Synths', icon: SynthIcon },
  samples: { name: 'Sample Banks', icon: CircleStackIcon },
  effects: { name: 'Effects', icon: SparklesIcon },
  sequences: { name: 'Sequences', icon: ArrowPathIcon },
  techniques: { name: 'Techniques', icon: BoltIcon },
  mininotation: { name: 'Mini Notation', icon: DocumentTextIcon },
  manipulation: { name: 'Sample Tricks', icon: WrenchScrewdriverIcon },
  scales: { name: 'Scales & Harmony', icon: ScaleIcon },
  funky: { name: 'Funky Grooves', icon: UserIcon },
  melodies: { name: 'Melody Ideas', icon: MelodyIcon },
  complete: { name: 'Full Patterns', icon: CompleteIcon },
  famous: { name: 'Famous Tunes', icon: StarIcon },
  videogame: { name: 'Video Game Music', icon: ComputerDesktopIcon },
  jazz: { name: 'Jazz Patterns', icon: JazzIcon },
  electronic: { name: 'Electronic', icon: ElectronicIcon },
  world: { name: 'World Music', icon: GlobeAltIcon },
};

// Example snippets collection
const snippets = [
  // ===== DRUMS & BEATS =====
  {
    id: 'drums-basic',
    category: 'drums',
    name: 'Basic Drum Pattern',
    description: 'Simple kick, snare, hi-hat pattern',
    code: `stack(
  s("bd sd bd sd"),
  s("hh*8").gain(0.6),
  s("~ cp ~ cp").gain(0.4)
)`,
  },
  {
    id: 'drums-techno',
    category: 'drums',
    name: 'Techno Beat',
    description: 'Four-on-the-floor techno',
    code: `stack(
  s("bd*4"),
  s("~ sd ~ sd").gain(0.8),
  s("[~ hh]*4").gain(0.6)
)`,
  },
  {
    id: 'drums-tr909',
    category: 'drums',
    name: 'Roland TR-909',
    description: 'Classic 909 drum machine',
    code: `s("bd*2, ~ [cp,sd]").bank('RolandTR909')`,
  },
  {
    id: 'drums-breakbeat',
    category: 'drums',
    name: 'Breakbeat Pattern',
    description: 'Funky breakbeat groove',
    code: `s("[bd sd]*2, hh*8, [~ oh ~]*2").gain(0.8)`,
  },
  {
    id: 'drums-jungle',
    category: 'drums',
    name: 'Jungle Break',
    description: 'Fast jungle drum pattern',
    code: `s("amencutup").n("<0 1 2 3 4 5 6 7>*2").fast(2)`,
  },

  // ===== PERCUSSION =====
  {
    id: 'perc-tabla',
    category: 'percussion',
    name: 'Tabla Pattern',
    description: 'Indian tabla drums',
    code: `s("tabla:0 ~ tabla:2 tabla:1")`,
  },
  {
    id: 'perc-conga',
    category: 'percussion',
    name: 'Conga Groove',
    description: 'Latin conga rhythm',
    code: `s("conga:0*2 conga:1 conga:2 conga:1")`,
  },
  {
    id: 'perc-bongos',
    category: 'percussion',
    name: 'Bongos',
    description: 'Bongo pattern',
    code: `s("bongo:0 bongo:1 ~ bongo:0")`,
  },
  {
    id: 'perc-shaker',
    category: 'percussion',
    name: 'Shaker Pattern',
    description: 'Continuous shaker groove',
    code: `s("shaker*16").gain("<0.5 0.8 0.5 0.6>")`,
  },
  {
    id: 'melodic-notes',
    category: 'melodic',
    name: 'Melodic Pattern',
    description: 'Echo pattern with harmonies',
    code: `note("<0 2 [4 6]>(3,4,2) 3*2>")
  .off(1/4, x=>x.add(2))
  .off(1/2, x=>x.add(6))
  .scale('D:minor')
  .s("sawtooth")
  .gain(0.8)`,
  },
  {
    id: 'melodic-bass',
    category: 'melodic',
    name: 'Bass Groove',
    description: 'Deep bass line',
    code: `note("<c2 eb2 f2 g2>")
  .s("sawtooth")
  .lpf(800)
  .gain(0.8)`,
  },
  {
    id: 'melodic-piano',
    category: 'melodic',
    name: 'Electric Piano',
    description: 'Jazzy electric piano chords',
    code: `note("<C^7 Dm7 G7 C^7>").voicing().s("gm_epiano1")`,
  },
  {
    id: 'melodic-arpy',
    category: 'melodic',
    name: 'Arpeggio (Arpy)',
    description: 'Classic arpy sample',
    code: `s("arpy").n("<0 2 4 7>").gain(0.8)`,
  },
  {
    id: 'melodic-casio',
    category: 'melodic',
    name: 'Casio Melody',
    description: 'Casio keyboard sound',
    code: `s("casio").n("0 2 4 5").slow(2)`,
  },
  {
    id: 'melodic-pluck',
    category: 'melodic',
    name: 'Plucked String',
    description: 'Pluck sample with reverb',
    code: `s("pluck").n("<0 2 4 7>").room(0.5)`,
  },

  // ===== BASS =====
  {
    id: 'bass-808',
    category: 'bass',
    name: '808 Bass',
    description: 'Classic 808 sub bass',
    code: `s("bass808").n("<0 2 3 2>").gain(0.9)`,
  },
  {
    id: 'bass-synth',
    category: 'bass',
    name: 'Synth Bass',
    description: 'Filtered sawtooth bass',
    code: `note("<c2 eb2 f2 g2>")
  .s("sawtooth")
  .lpf(400)
  .resonance(10)
  .gain(0.8)`,
  },
  {
    id: 'bass-square',
    category: 'bass',
    name: 'Square Wave Bass',
    description: 'Aggressive square bass',
    code: `note("<c2 g1 bb1 f2>").s("square").lpf(600).decay(0.1)`,
  },
  {
    id: 'bass-reese',
    category: 'bass',
    name: 'Reese Bass',
    description: 'Detuned bass sound',
    code: `note("c2").s("bass3").gain(0.9).room(0.3)`,
  },

  // ===== SYNTHS =====
  {
    id: 'synth-sawtooth',
    category: 'synth',
    name: 'Sawtooth Wave',
    description: 'Classic sawtooth synth',
    code: `note("<c4 e4 g4 a4>").s("sawtooth").lpf(1200)`,
  },
  {
    id: 'synth-square',
    category: 'synth',
    name: 'Square Wave',
    description: 'Hollow square wave',
    code: `note("<c4 e4 g4 a4>").s("square").gain(0.7)`,
  },
  {
    id: 'synth-triangle',
    category: 'synth',
    name: 'Triangle Wave',
    description: 'Smooth triangle wave',
    code: `note("<c4 e4 g4 a4>").s("triangle").gain(0.8)`,
  },
  {
    id: 'synth-sine',
    category: 'synth',
    name: 'Sine Wave',
    description: 'Pure sine wave',
    code: `note("<c4 e4 g4 a4>").s("sine").gain(0.8)`,
  },
  {
    id: 'synth-pad',
    category: 'synth',
    name: 'Synth Pad',
    description: 'Lush pad sound',
    code: `note("<C^7 Dm7 G7 C^7>")
  .voicing()
  .s("sawtooth")
  .lpf(800)
  .room(0.8)
  .slow(2)`,
  },
  {
    id: 'synth-lead',
    category: 'synth',
    name: 'Lead Synth',
    description: 'Bright lead sound',
    code: `note("<c5 d5 e5 g5>")
  .s("square")
  .lpf(2000)
  .resonance(5)
  .decay(0.2)`,
  },

  // ===== SAMPLE BANKS =====
  {
    id: 'samples-breath',
    category: 'samples',
    name: 'Breath Sounds',
    description: 'Atmospheric breath',
    code: `s("breath").chop(16).rev().room(1)`,
  },
  {
    id: 'samples-metal',
    category: 'samples',
    name: 'Metal Hits',
    description: 'Metallic percussion',
    code: `s("metal").n("<0 1 2 3>").gain(0.7)`,
  },
  {
    id: 'samples-jazz',
    category: 'samples',
    name: 'Jazz Kit',
    description: 'Jazz drum samples',
    code: `s("jazz:0 jazz:1 jazz:2 jazz:3")`,
  },
  {
    id: 'samples-east',
    category: 'samples',
    name: 'East Sounds',
    description: 'Eastern percussion',
    code: `s("east").n("<0 1 2>").delay(0.5)`,
  },
  {
    id: 'samples-gretsch',
    category: 'samples',
    name: 'Gretsch Drums',
    description: 'Gretsch drum kit',
    code: `s("gretsch:0*2 gretsch:1 gretsch:2*2")`,
  },
  {
    id: 'effects-echo',
    category: 'effects',
    name: 'Echo Effect',
    description: 'Add echo/delay',
    code: `.echo(4, 1/8, 0.5)`,
  },
  {
    id: 'effects-reverb',
    category: 'effects',
    name: 'Reverb',
    description: 'Add room reverb',
    code: `.room(0.5).size(0.8)`,
  },
  {
    id: 'effects-filter',
    category: 'effects',
    name: 'Low Pass Filter',
    description: 'Sweep low-pass filter',
    code: `.lpf("<400 800 1600 3200>")
  .resonance(10)`,
  },
  {
    id: 'effects-hpf',
    category: 'effects',
    name: 'High Pass Filter',
    description: 'Cut low frequencies',
    code: `.hpf(1000).resonance(5)`,
  },
  {
    id: 'effects-crush',
    category: 'effects',
    name: 'Bit Crusher',
    description: 'Lo-fi bit crushing',
    code: `.crush("<4 6 8 16>")`,
  },
  {
    id: 'effects-distortion',
    category: 'effects',
    name: 'Distortion',
    description: 'Overdrive/distortion',
    code: `.shape(0.8).gain(0.5)`,
  },
  {
    id: 'effects-vowel',
    category: 'effects',
    name: 'Vowel Filter',
    description: 'Formant filter',
    code: `.vowel("<a e i o u>")`,
  },
  {
    id: 'effects-phaser',
    category: 'effects',
    name: 'Phaser Effect',
    description: 'Sweeping phaser',
    code: `.phaser("<4 8 16>").phaserDepth(0.5)`,
  },
  {
    id: 'sequences-euclidean',
    category: 'sequences',
    name: 'Euclidean Rhythm',
    description: 'Evenly distributed hits',
    code: `s("bd(3,8)").bank("RolandTR909")`,
  },
  {
    id: 'sequences-polyrhythm',
    category: 'sequences',
    name: 'Polyrhythm',
    description: 'Multiple time divisions',
    code: `stack(
  s("bd(5,8)"),
  s("sd(3,8)"),
  s("hh(7,8)").gain(0.6)
)`,
  },
  {
    id: 'sequences-polymeter',
    category: 'sequences',
    name: 'Polymeter',
    description: 'Different pattern lengths',
    code: `stack(
  note("c3 d3 e3").slow(3),
  note("g4 a4").slow(2)
)`,
  },
  {
    id: 'sequences-struct',
    category: 'sequences',
    name: 'Struct Pattern',
    description: 'Apply rhythm structure',
    code: `note("<0 2 4 7>").struct("x x ~ x")`,
  },
  {
    id: 'techniques-offbeat',
    category: 'techniques',
    name: 'Offbeat Pattern',
    description: 'Create harmony with offset',
    code: `.off(1/4, x => x.add(7))
  .off(1/2, x => x.add(12))`,
  },
  {
    id: 'techniques-fast',
    category: 'techniques',
    name: 'Speed Up',
    description: 'Double the tempo',
    code: `.fast(2)`,
  },
  {
    id: 'techniques-every',
    category: 'techniques',
    name: 'Pattern Variation',
    description: 'Every Nth cycle, transform',
    code: `.every(4, fast(2))
  .every(8, x => x.rev())`,
  },
  {
    id: 'techniques-jux',
    category: 'techniques',
    name: 'Jux (Stereo Split)',
    description: 'Different versions in each ear',
    code: `.jux(rev)`,
  },
  {
    id: 'techniques-juxby',
    category: 'techniques',
    name: 'Jux By Amount',
    description: 'Controlled stereo split',
    code: `.juxBy(0.5, x => x.fast(2))`,
  },
  {
    id: 'techniques-iter',
    category: 'techniques',
    name: 'Iter (Rotate)',
    description: 'Rotate pattern each cycle',
    code: `note("0 2 4 7").iter(4)`,
  },
  {
    id: 'techniques-chunk',
    category: 'techniques',
    name: 'Chunk Transform',
    description: 'Transform parts of pattern',
    code: `.chunk(4, x => x.fast(2))`,
  },
  {
    id: 'techniques-sometimes',
    category: 'techniques',
    name: 'Sometimes',
    description: 'Random transformation',
    code: `.sometimes(fast(2))`,
  },
  {
    id: 'techniques-degradeby',
    category: 'techniques',
    name: 'Degrade By',
    description: 'Randomly remove events',
    code: `.degradeBy(0.3)`,
  },
  {
    id: 'techniques-palindrome',
    category: 'techniques',
    name: 'Palindrome',
    description: 'Play forward then reverse',
    code: `.palindrome()`,
  },

  // ===== MINI NOTATION =====
  {
    id: 'mini-rests',
    category: 'mininotation',
    name: 'Rests (~)',
    description: 'Silence in pattern',
    code: `s("bd ~ sd ~")`,
  },
  {
    id: 'mini-subdivision',
    category: 'mininotation',
    name: 'Subdivisions []',
    description: 'Faster events in brackets',
    code: `s("bd [hh hh hh hh] sd hh")`,
  },
  {
    id: 'mini-multiply',
    category: 'mininotation',
    name: 'Multiply (*)',
    description: 'Repeat event N times',
    code: `s("bd*4 sd*2")`,
  },
  {
    id: 'mini-divide',
    category: 'mininotation',
    name: 'Divide (/)',
    description: 'Slow down by N',
    code: `s("bd/2 sd")`,
  },
  {
    id: 'mini-alternate',
    category: 'mininotation',
    name: 'Alternate <>',
    description: 'Cycle through options',
    code: `s("<bd cp sd>")`,
  },
  {
    id: 'mini-stack',
    category: 'mininotation',
    name: 'Stack (,)',
    description: 'Play simultaneously',
    code: `s("[bd, hh*8, ~ sd ~]")`,
  },
  {
    id: 'mini-sample-number',
    category: 'mininotation',
    name: 'Sample Numbers',
    description: 'Access sample variants',
    code: `s("arpy:0 arpy:1 arpy:2")`,
  },
  {
    id: 'mini-elongate',
    category: 'mininotation',
    name: 'Elongate (@)',
    description: 'Hold event longer',
    code: `s("bd@3 sd")`,
  },
  {
    id: 'mini-replicate',
    category: 'mininotation',
    name: 'Replicate (!)',
    description: 'Repeat subsequence',
    code: `s("[bd sd]!4")`,
  },

  // ===== SAMPLE MANIPULATION =====
  {
    id: 'manip-speed',
    category: 'manipulation',
    name: 'Speed Change',
    description: 'Pitch and speed',
    code: `.speed("<1 0.5 2 1.5>")`,
  },
  {
    id: 'manip-chop',
    category: 'manipulation',
    name: 'Chop Sample',
    description: 'Slice into pieces',
    code: `.chop(16)`,
  },
  {
    id: 'manip-striate',
    category: 'manipulation',
    name: 'Striate',
    description: 'Granular effect',
    code: `.striate(32)`,
  },
  {
    id: 'manip-begin-end',
    category: 'manipulation',
    name: 'Begin/End',
    description: 'Play part of sample',
    code: `.begin(0.25).end(0.75)`,
  },
  {
    id: 'manip-cut',
    category: 'manipulation',
    name: 'Cut Group',
    description: 'Stop previous same sound',
    code: `.cut(1)`,
  },
  {
    id: 'manip-loopat',
    category: 'manipulation',
    name: 'Loop At Speed',
    description: 'Loop to fit cycles',
    code: `.loopAt(4)`,
  },
  {
    id: 'manip-legato',
    category: 'manipulation',
    name: 'Legato',
    description: 'Note length as fraction',
    code: `.legato("<0.5 1 2>")`,
  },
  {
    id: 'manip-coarse',
    category: 'manipulation',
    name: 'Coarse Pitch',
    description: 'Semitone transpose',
    code: `.coarse("<0 5 7 12>")`,
  },

  // ===== SCALES & HARMONY =====
  {
    id: 'scales-minor',
    category: 'scales',
    name: 'Minor Scale',
    description: 'Natural minor scale',
    code: `note("0 2 3 5 7 8 10")
  .scale("C:minor")`,
  },
  {
    id: 'scales-pentatonic',
    category: 'scales',
    name: 'Pentatonic Scale',
    description: 'Minor pentatonic',
    code: `note("0 2 3 5 7")
  .scale("C:minor:pentatonic")`,
  },
  {
    id: 'scales-bebop',
    category: 'scales',
    name: 'Bebop Scale',
    description: 'Jazz bebop scale',
    code: `note("0 2 3 5 7 8 10 11")
  .scale("C:bebop:major")`,
  },
  {
    id: 'scales-blues',
    category: 'scales',
    name: 'Blues Scale',
    description: 'Blues scale with blue note',
    code: `note("0 3 5 6 7 10")
  .scale("C:blues")`,
  },
  {
    id: 'scales-dorian',
    category: 'scales',
    name: 'Dorian Mode',
    description: 'Dorian mode (jazzy)',
    code: `note("0 2 3 5 7 9 10")
  .scale("C:dorian")`,
  },
  {
    id: 'scales-mixolydian',
    category: 'scales',
    name: 'Mixolydian Mode',
    description: 'Dominant 7th mode',
    code: `note("0 2 4 5 7 9 10")
  .scale("C:mixolydian")`,
  },
  {
    id: 'harmony-chords',
    category: 'scales',
    name: 'Chord Progression',
    description: 'Jazz ii-V-I progression',
    code: `chord("<Dm7 G7 C^7>")
  .voicing()
  .note()`,
  },
  {
    id: 'harmony-voicing',
    category: 'scales',
    name: 'Lefthand Voicing',
    description: 'Jazz piano voicings',
    code: `chord("C^7").dict('lefthand').voicing()`,
  },
  {
    id: 'harmony-rootnotes',
    category: 'scales',
    name: 'Root Notes',
    description: 'Extract bass notes from chords',
    code: `chord("<Dm7 G7 C^7>").rootNotes(2)`,
  },

  // ===== FUNKY GROOVES =====
  {
    id: 'funky-breakbeat',
    category: 'funky',
    name: 'Funky Breakbeat',
    description: 'Classic breakbeat groove',
    code: `stack(
  s("bd*2, ~ [cp,sd]").gain(0.8),
  s("hh*8").gain(0.5),
  s("~ ~ oh ~").gain(0.6)
)`,
  },
  {
    id: 'funky-bass-groove',
    category: 'funky',
    name: 'Funk Bass Line',
    description: 'Slap bass groove',
    code: `note("c2 ~ c3 ~ eb2 ~ c2 ~")
  .s("sawtooth")
  .lpf(400)
  .decay(0.1)
  .gain(0.8)`,
  },
  {
    id: 'funky-disco',
    category: 'funky',
    name: 'Disco Beat',
    description: 'Four-on-floor disco',
    code: `stack(
  s("bd*4"),
  s("~ oh*2 ~").gain(0.7),
  s("hh*8").gain(0.4)
).fast(1.2)`,
  },
  {
    id: 'funky-sidestick',
    category: 'funky',
    name: 'Sidestick Groove',
    description: 'Funky sidestick pattern',
    code: `s("~ [cp,rim] ~ cp").gain(0.8)`,
  },
  {
    id: 'funky-hihat-16th',
    category: 'funky',
    name: '16th Note Hi-Hats',
    description: 'Fast funky hi-hats',
    code: `s("hh*16")
  .gain("<0.6 0.8 0.5 0.7>*4")
  .lpf(4000)`,
  },
  {
    id: 'funky-chopped',
    category: 'funky',
    name: 'Chopped Funk',
    description: 'Chopped sample funk',
    code: `s("jazz").chop(8).sometimes(rev)`,
  },
  {
    id: 'funky-amen',
    category: 'funky',
    name: 'Amen Break',
    description: 'Jungle amen break',
    code: `s("amencutup")
  .n("<0 1 2 3 4 5 6 7>*2")
  .fast(2)
  .sometimes(rev)`,
  },

  // ===== MELODY IDEAS =====
  {
    id: 'melody-arp-up',
    category: 'melodies',
    name: 'Arpeggio Up',
    description: 'Rising arpeggio pattern',
    code: `note("c4 e4 g4 c5")
  .s("triangle")
  .room(0.5)`,
  },
  {
    id: 'melody-arp-down',
    category: 'melodies',
    name: 'Arpeggio Down',
    description: 'Falling arpeggio',
    code: `note("c5 g4 e4 c4")
  .s("sawtooth")
  .lpf(1200)`,
  },
  {
    id: 'melody-echo-harmony',
    category: 'melodies',
    name: 'Echo Harmony',
    description: 'Melody with echoed harmonies',
    code: `note("0 2 [4 6](3,4) 3*2")
  .off(1/4, x=>x.add(2))
  .off(1/2, x=>x.add(7))
  .scale('D:minor')`,
  },
  {
    id: 'melody-octave-jump',
    category: 'melodies',
    name: 'Octave Jumps',
    description: 'Jump between octaves',
    code: `note("<c4 c5>*4")
  .off(1/8, x=>x.add(7))
  .s("square")`,
  },
  {
    id: 'melody-pentatonic-run',
    category: 'melodies',
    name: 'Pentatonic Run',
    description: 'Fast pentatonic scale run',
    code: `note("0 2 3 5 7 5 3 2")
  .scale("C:minor:pentatonic")
  .fast(2)`,
  },
  {
    id: 'melody-chord-stabs',
    category: 'melodies',
    name: 'Chord Stabs',
    description: 'Staccato chord hits',
    code: `chord("<C^7 Dm7 G7>")
  .voicing()
  .struct("x ~ x ~")
  .clip(0.1)`,
  },
  {
    id: 'melody-bass-melody',
    category: 'melodies',
    name: 'Bass Melody',
    description: 'Melodic bass line',
    code: `note("c2 ~ eb2 f2 ~ g2 ~ f2")
  .s("sawtooth")
  .lpf(600)`,
  },
  {
    id: 'melody-call-response',
    category: 'melodies',
    name: 'Call and Response',
    description: 'Question-answer phrase',
    code: `note("<[c4 e4 g4] [d4 f4 a4]>")
  .slow(2)
  .room(0.5)`,
  },
  {
    id: 'melody-scale-run',
    category: 'melodies',
    name: 'Scale Run',
    description: 'Fast scale run up and down',
    code: `note("0 1 2 3 4 5 6 7 6 5 4 3 2 1")
  .scale("C:major")
  .fast(2)`,
  },

  // ===== FULL PATTERNS =====
  {
    id: 'complete-four-floor',
    category: 'complete',
    name: 'Four-on-Floor',
    description: 'Classic four-on-floor beat',
    code: `stack(
  s("bd*4"),
  s("~ sd ~ sd"),
  s("hh*8").gain(0.6)
)`,
  },
  {
    id: 'complete-boom-bap',
    category: 'complete',
    name: 'Boom Bap',
    description: 'Hip-hop boom bap',
    code: `stack(
  s("bd ~ sd ~"),
  s("hh*4").gain(0.5)
)`,
  },
  {
    id: 'complete-ambient',
    category: 'complete',
    name: 'Ambient Pad',
    description: 'Slow evolving pads',
    code: `chord("<C^7 Am7 F^7 G7>")
  .voicing()
  .s("sawtooth")
  .lpf(800)
  .room(0.9)
  .slow(4)`,
  },
  {
    id: 'complete-euclidean',
    category: 'complete',
    name: 'Euclidean Pattern',
    description: 'Polyrhythmic euclidean',
    code: `stack(
  s("bd(3,8)"),
  s("sd(5,8)"),
  s("hh(7,8)").gain(0.5)
)`,
  },
  {
    id: 'complete-bass-and-drums',
    category: 'complete',
    name: 'Bass + Drums',
    description: 'Drums with bassline',
    code: `stack(
  s("bd ~ sd ~"),
  s("hh*8").gain(0.5),
  note("c2 ~ eb2 ~")
    .s("sawtooth")
    .lpf(600)
)`,
  },

  // ===== MORE DRUMS =====
  {
    id: 'drums-electro',
    category: 'drums',
    name: 'Electro Beat',
    description: '80s electro drum pattern',
    code: `stack(
  s("bd ~ bd ~"),
  s("~ sd ~ sd").gain(0.9),
  s("~ ~ hh ~").gain(0.6)
).bank('RolandTR808')`,
  },
  {
    id: 'drums-halftime',
    category: 'drums',
    name: 'Half-Time Beat',
    description: 'Slow hip-hop halftime',
    code: `s("bd ~ ~ ~ sd ~ ~ ~")
  .stack(s("hh*8").gain(0.5))
  .slow(2)`,
  },
  {
    id: 'drums-polyrhythm',
    category: 'drums',
    name: 'Polyrhythmic Drums',
    description: 'Complex polyrhythm',
    code: `stack(
  s("bd(5,8)"),
  s("sd(3,8)"),
  s("hh(7,8)").gain(0.5)
)`,
  },

  // ===== MORE EFFECTS =====
  {
    id: 'effects-djf',
    category: 'effects',
    name: 'DJ Filter',
    description: 'Filter sweep like DJ mixer',
    code: `.djf(sine.range(0,1).slow(4))`,
  },
  {
    id: 'effects-leslie',
    category: 'effects',
    name: 'Leslie Speaker',
    description: 'Rotary speaker effect',
    code: `.leslie(4).lesliespeed(0.5)`,
  },
  {
    id: 'effects-tremolo',
    category: 'effects',
    name: 'Tremolo',
    description: 'Amplitude modulation',
    code: `.tremolo("<4 8>")`,
  },
  {
    id: 'effects-autopan',
    category: 'effects',
    name: 'Auto Pan',
    description: 'Stereo auto-panning',
    code: `.pan(sine.slow(4))`,
  },
  {
    id: 'effects-stereo-spread',
    category: 'effects',
    name: 'Stereo Spread',
    description: 'Widen stereo image',
    code: `.juxBy(0.5, x=>x.add(0.01))`,
  },

  // ===== MORE TECHNIQUES =====
  {
    id: 'techniques-mask',
    category: 'techniques',
    name: 'Pattern Mask',
    description: 'Conditionally silence events',
    code: `.mask("<x@7 ~>/8")`,
  },
  {
    id: 'techniques-struct',
    category: 'techniques',
    name: 'Struct Rhythm',
    description: 'Apply rhythmic structure',
    code: `.struct("x ~ x [x x]")`,
  },
  {
    id: 'techniques-anchor',
    category: 'techniques',
    name: 'Anchor Pattern',
    description: 'Sync with another pattern',
    code: `.anchor(melody)`,
  },
  {
    id: 'techniques-layer',
    category: 'techniques',
    name: 'Layer Transform',
    description: 'Create multiple layers',
    code: `.layer(
  x=>x.add(0),
  x=>x.add(7),
  x=>x.add(12)
)`,
  },
  {
    id: 'techniques-ply',
    category: 'techniques',
    name: 'Ply (Subdivide)',
    description: 'Repeat each event',
    code: `.ply(2)`,
  },
  {
    id: 'techniques-segment',
    category: 'techniques',
    name: 'Segment',
    description: 'Divide pattern into segments',
    code: `.segment(4)`,
  },
  {
    id: 'techniques-range',
    category: 'techniques',
    name: 'Range (Signal)',
    description: 'Map signal to range',
    code: `.lpf(sine.range(400,2000).slow(4))`,
  },
  {
    id: 'techniques-perlin',
    category: 'techniques',
    name: 'Perlin Noise',
    description: 'Smooth random modulation',
    code: `.gain(perlin.range(0.6,0.9))`,
  },

  // ===== FAMOUS TUNES =====
  {
    id: 'famous-giant-steps',
    category: 'famous',
    name: 'Giant Steps',
    description: 'John Coltrane - Giant Steps',
    code: `// John Coltrane - Giant Steps
let melody = seq(
  "[F#5 D5] [B4 G4] Bb4 [B4 A4]",
  "[D5 Bb4] [G4 Eb4] F#4 [G4 F4]",
  "Bb4 [B4 A4] D5 [D#5 C#5]",
  "F#5 [G5 F5] Bb5 [F#5 F#5]",
).note()

stack(
  melody,
  seq(
    "[B^7 D7] [G^7 Bb7] Eb^7 [Am7 D7]",
    "[G^7 Bb7] [Eb^7 F#7] B^7 [Fm7 Bb7]",
    "Eb^7 [Am7 D7] G^7 [C#m7 F#7]",
    "B^7 [Fm7 Bb7] Eb^7 [C#m7 F#7]"
  ).chord().dict('lefthand')
  .anchor(melody).mode('duck')
  .voicing()
).slow(20)`,
  },
  {
    id: 'famous-barry-harris',
    category: 'famous',
    name: 'Barry Harris Exercise',
    description: 'Bebop exercise',
    code: `// adapted from a Barry Harris excercise
"0,2,[7 6]"
  .add("<0 1 2 3 4 5 7 8>")
  .scale('C:bebop:major')
  .transpose("<0 1 2 1>/8")
  .slow(2)
  .note().piano()`,
  },
  {
    id: 'famous-echopiano',
    category: 'famous',
    name: 'Echo Piano',
    description: 'by Felix Roos',
    code: `// "Echo piano" by Felix Roos
n("<0 2 [4 6](3,4,2) 3*2>")
.off(1/4, x=>x.add(n(2)))
.off(1/2, x=>x.add(n(6)))
.scale('D:minor')
.echo(4, 1/8, .5)
.clip(.5)
.piano()`,
  },
  {
    id: 'famous-good-times',
    category: 'famous',
    name: 'Good Times',
    description: 'by Felix Roos',
    code: `// "Good times" by Felix Roos
const scale = cat('C3:dorian','Bb2:major').slow(4);
stack(
  n("2*4".add(12)).off(1/8, add(2))
    .scale(scale)
    .fast(2)
    .add("<0 1 2 1>"),
  "<0 1 2 3>(3,8,2)".off(1/4, add("2,4"))
    .n().scale(scale),
  n("<0 4>(5,8,-1)").scale(scale).sub(note(12))
)
  .gain(".6 .7".fast(4))
  .add(note(4))
  .piano()
  .slow(2)`,
  },
  {
    id: 'famous-blue-monday',
    category: 'famous',
    name: 'Blue Monday',
    description: 'New Order (simplified)',
    code: `// Blue Monday - New Order
stack(
  s("bd!2 [bd*4]!2 bd!4").slow(8)
    .bank("SequentialCircuitsDrumtracks"),
  s("~ hh").bank("SequentialCircuitsDrumtracks"),
  n("<[[2 ~] [2 ~] 2 3] [[3 ~] [3 ~] 3 3]>@4")
    .slow(8)
    .scale("d2:minor")
    .s("sawtooth")
).cpm(130)`,
  },
  {
    id: 'famous-enjoy-silence',
    category: 'famous',
    name: 'Enjoy the Silence',
    description: 'Depeche Mode (simplified)',
    code: `// Enjoy the Silence - Depeche Mode
const scala = cat('c:minor')
stack(
  "<[3,5,0] [5,0,2] [0,2,4] [2,4,-1]>"
    .scale(scala)
    .voicing()
    .note(),
  "<[2@3 3] [0@3 2] [4@3 6] [2@3 3]>"
    .scale(scala)
    .transpose(12)
    .note()
    .s("triangle"),
  s("bd!4,[~ sd]!2,[~ hh!2 hh*2]!2")
    .bank("AlesisHR16")
    .gain(0.5)
)`,
  },
  {
    id: 'famous-happy-birthday',
    category: 'famous',
    name: 'Happy Birthday',
    description: 'Traditional',
    code: `// Happy Birthday
const chrds = "F@3 C@6 F@6 Bb@3 F@2 C F@3".slow(8);
stack(
  "[C4@3 C4] D4 C4 F4 E4@2 [C4@3 C4] D4 C4 G4 F4@2"
    .slow(8)
    .note()
    .s("triangle"),
  chord(chrds)
    .anchor("G4")
    .struct("x*3")
    .voicing(),
  s("hh*3, <bd ~>, ~ ~ rim")
    .bank("KorgDDM110")
    .gain(0.2)
).cpm(120/4).room(0.3)`,
  },
  {
    id: 'famous-stranger-things',
    category: 'famous',
    name: 'Stranger Things Theme',
    description: 'Kyle Dixon & Michael Stein',
    code: `// Stranger Things Theme
stack(
  n("0 2 4 6 7 6 4 2")
    .scale("c3:major")
    .s("sawtooth")
    .lpf(perlin.slow(2).range(100,2000)),
  "<a1 e2>/8"
    .clip(0.8)
    .struct("x*8")
    .s("sawtooth")
    .note()
).slow(2)`,
  },

  // ===== VIDEO GAME MUSIC =====
  {
    id: 'vgm-mario-1-1',
    category: 'videogame',
    name: 'Super Mario Bros 1-1',
    description: 'Hirokazu Tanaka - World 1-1',
    code: `// Hirokazu Tanaka - World 1-1 (simplified)
note("e5 e5 ~ e5 ~ c5 e5 ~ g5 ~ ~ ~ g4")
  .s('square')
  .clip(.95)
  .fast(2)`,
  },
  {
    id: 'vgm-mario-swimming',
    category: 'videogame',
    name: 'Super Mario Swimming',
    description: 'Koji Kondo - Swimming',
    code: `// Koji Kondo - Swimming (simplified)
note("A5 [F5@2 C5] [D5@2 F5] F5")
  .s('triangle')
  .gain(.1)
  .room(1)
  .slow(4)`,
  },
  {
    id: 'vgm-zelda-rescue',
    category: 'videogame',
    name: "Zelda's Rescue",
    description: 'Koji Kondo',
    code: `// Koji Kondo - Princess Zelda's Rescue (simplified)
note("[B3@2 D4] [A3@2 [G3 A3]] [B3@2 D4] [A3]")
  .transpose(12)
  .s('triangle')
  .gain(.1)
  .room(1)
  .slow(4)`,
  },
  {
    id: 'vgm-chiptune-bass',
    category: 'videogame',
    name: 'Chiptune Bass',
    description: 'Square wave bass line',
    code: `note("c2 ~ c2 ~ eb2 ~ c2 f2")
  .s('square')
  .lpf(400)`,
  },
  {
    id: 'vgm-chip-arp',
    category: 'videogame',
    name: 'Chip Arpeggio',
    description: 'Fast arpeggio',
    code: `note("c5 e5 g5 c6")
  .s('square')
  .decay(0.1)
  .fast(4)`,
  },

  // ===== JAZZ PATTERNS =====
  {
    id: 'jazz-251',
    category: 'jazz',
    name: 'ii-V-I Progression',
    description: 'Classic jazz progression',
    code: `chord("<Dm7 G7 C^7>")
  .voicing()
  .dict('lefthand')`,
  },
  {
    id: 'jazz-walking-bass',
    category: 'jazz',
    name: 'Walking Bass',
    description: 'Jazz bass walking',
    code: `note("c2 e2 g2 bb2")
  .s('sawtooth')
  .lpf(600)`,
  },
  {
    id: 'jazz-comping',
    category: 'jazz',
    name: 'Jazz Comping',
    description: 'Piano comping',
    code: `chord("<C^7 Am7 Dm7 G7>")
  .voicing()
  .struct("~ x ~ x")
  .clip(0.2)`,
  },

  // ===== ELECTRONIC =====
  {
    id: 'electronic-acid-303',
    category: 'electronic',
    name: 'TB-303 Acid Bass',
    description: 'Classic acid bass',
    code: `note("[c2 eb2 g2 bb2](5,8)")
  .s('sawtooth')
  .lpf(sine.range(400,2000).slow(4))
  .lpq(10)
  .decay(0.1)`,
  },
  {
    id: 'electronic-wobble',
    category: 'electronic',
    name: 'Dubstep Wobble',
    description: 'LFO wobble bass',
    code: `note("c2")
  .s('sawtooth')
  .lpf(sine.range(100,2000).fast(8))
  .lpq(20)`,
  },
  {
    id: 'electronic-sidechain',
    category: 'electronic',
    name: 'Sidechain Pump',
    description: 'Fake sidechain',
    code: `.gain("[.2 1@3]*2")`,
  },
  {
    id: 'electronic-riser',
    category: 'electronic',
    name: 'Riser Effect',
    description: 'Build-up sweep',
    code: `note("c3")
  .s('sawtooth')
  .lpf(sine.range(200,8000).slow(8))
  .gain(sine.range(0,1).slow(8))
  .slow(8)`,
  },

  // ===== WORLD MUSIC =====
  {
    id: 'world-bossa',
    category: 'world',
    name: 'Bossa Nova',
    description: 'Brazilian bossa rhythm',
    code: `stack(
  s("bd ~ ~ bd ~ bd ~"),
  s("rim ~ rim ~ rim ~ rim ~")
).slow(1.5)`,
  },
  {
    id: 'world-samba',
    category: 'world',
    name: 'Samba',
    description: 'Brazilian samba',
    code: `stack(
  s("bd*4"),
  s("~ sd ~ [sd sd]")
).fast(1.8)`,
  },
  {
    id: 'world-tabla',
    category: 'world',
    name: 'Tabla Pattern',
    description: 'Indian tabla',
    code: `s("tabla:0 ~ tabla:2 tabla:1")`,
  },
  {
    id: 'world-reggae',
    category: 'world',
    name: 'Reggae Skank',
    description: 'Reggae chords',
    code: `chord("<C Am F G>")
  .voicing()
  .struct("~ x ~ x")
  .clip(0.1)`,
  },
];

export function SnippetsTab({ context }) {
  const { fontFamily } = useSettings();
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState(null);

  const handleInsertSnippet = (snippet) => {
    if (context?.editorRef?.current) {
      const editor = context.editorRef.current;
      const cursor = editor.getCursorLocation();
      const currentCode = editor.code || '';

      // Insert at cursor position
      const beforeCursor = currentCode.substring(0, cursor);
      const afterCursor = currentCode.substring(cursor);
      const newCode = beforeCursor + snippet.code + afterCursor;

      editor.setCode(newCode);
      // Move cursor to end of inserted snippet
      editor.setCursorLocation(cursor + snippet.code.length);
    }
  };

  const filteredSnippets = snippets.filter((snippet) => {
    const matchesSearch =
      !searchQuery ||
      snippet.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      snippet.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      snippet.code.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesCategory = !selectedCategory || snippet.category === selectedCategory;
    return matchesSearch && matchesCategory;
  });

  const groupedSnippets = {};
  filteredSnippets.forEach((snippet) => {
    if (!groupedSnippets[snippet.category]) {
      groupedSnippets[snippet.category] = [];
    }
    groupedSnippets[snippet.category].push(snippet);
  });

  return (
    <div className="flex flex-col h-full" style={{ fontFamily }}>
      {/* Search and Filter */}
      <div className="p-3 border-b border-[var(--border-cyan)] bg-[rgba(34,211,238,0.05)]">
        <input
          type="text"
          placeholder="Search snippets..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-full px-3 py-2 bg-[#0f172a] border border-[var(--border-cyan)] rounded-md text-[var(--foreground)] placeholder-[var(--cyan-400)] placeholder-opacity-30 focus:outline-none focus:border-[var(--cyan-400)] font-mono text-sm"
        />
        <div className="flex gap-2 mt-2 flex-wrap">
          <button
            onClick={() => setSelectedCategory(null)}
            className={`px-2 py-1 rounded text-xs font-mono uppercase tracking-wider transition-all ${
              !selectedCategory
                ? 'bg-[var(--cyan-400)] text-[#0f172a]'
                : 'bg-[rgba(34,211,238,0.1)] text-[var(--cyan-400)] hover:bg-[rgba(34,211,238,0.2)]'
            }`}
          >
            All
          </button>
          {Object.entries(snippetCategories).map(([key, category]) => (
            <button
              key={key}
              onClick={() => setSelectedCategory(selectedCategory === key ? null : key)}
              className={`px-2 py-1 rounded text-xs font-mono uppercase tracking-wider transition-all flex items-center gap-1 ${
                selectedCategory === key
                  ? 'bg-[var(--cyan-400)] text-[#0f172a]'
                  : 'bg-[rgba(34,211,238,0.1)] text-[var(--cyan-400)] hover:bg-[rgba(34,211,238,0.2)]'
              }`}
            >
              {React.createElement(category.icon, { className: 'w-3 h-3' })}
              {category.name}
            </button>
          ))}
        </div>
      </div>

      {/* Snippets List */}
      <div className="flex-1 overflow-auto p-3">
        {Object.entries(groupedSnippets).length === 0 ? (
          <div className="text-center py-8 text-[var(--foreground)] opacity-50 font-mono text-sm">
            No snippets found
          </div>
        ) : (
          Object.entries(groupedSnippets).map(([categoryKey, categorySnippets]) => (
            <div key={categoryKey} className="mb-4">
              <h3 className="text-xs font-mono uppercase tracking-widest text-[var(--cyan-400)] mb-2 flex items-center gap-2">
                {React.createElement(snippetCategories[categoryKey].icon, { className: 'w-4 h-4' })}
                <span>{snippetCategories[categoryKey].name}</span>
                <span className="text-[var(--foreground)] opacity-30">({categorySnippets.length})</span>
              </h3>
              <div className="space-y-2">
                {categorySnippets.map((snippet) => (
                  <button
                    key={snippet.id}
                    onClick={() => handleInsertSnippet(snippet)}
                    className="w-full text-left p-3 rounded-md bg-[rgba(34,211,238,0.05)] border border-[var(--border-cyan)] hover:bg-[rgba(34,211,238,0.1)] hover:border-[var(--cyan-400)] transition-all group"
                  >
                    <div className="font-medium text-sm text-[var(--cyan-400)] mb-1">{snippet.name}</div>
                    <div className="text-xs text-[var(--foreground)] opacity-60 mb-2">{snippet.description}</div>
                    <pre className="text-xs font-mono text-[var(--foreground)] opacity-70 bg-[#0f172a] p-2 rounded overflow-x-auto group-hover:opacity-90 transition-opacity">
                      {snippet.code}
                    </pre>
                  </button>
                ))}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-2 border-t border-[var(--border-cyan)] bg-[rgba(34,211,238,0.05)]">
        <div className="flex items-center justify-between text-[10px] font-mono text-[var(--cyan-400)] opacity-50 uppercase tracking-wider">
          <span>{filteredSnippets.length} snippets</span>
          <span>Click to insert</span>
        </div>
      </div>
    </div>
  );
}
