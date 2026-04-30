/**
 * Letter ↔ animal mapping recovered from the Lumosity bundle's
 * `_InterestEnableSprites` array on the PetDetective board MonoBehaviour.
 *
 * In the on-disk level format, an UPPERCASE letter is the pet (animal) and
 * its matching lowercase letter is the house. Index 0 = A/a, … 10 = K/k.
 */
export interface AnimalEntry {
  /** zero-based index, 0..10. */
  index: number;
  /** uppercase letter as it appears in the level file (the pet). */
  letter: string;
  /** lowercase letter as it appears in the level file (the house). */
  homeLetter: string;
  /** human-readable animal name. */
  name: string;
  /** static URL to the pet sprite (vector-rendered PNG with transparent bg). */
  petSprite: string;
  /** static URL to the house texture. */
  houseSprite: string;
}

/**
 * Per-animal pet size aspect, taken from `InterestSettings_506.json` →
 * `interestSizeSettings.enableWidth/Height`. We keep their relative ratios
 * (dachshund is wider, cockatiel is taller, etc.) but scale to occupy ~75 %
 * of the 250 px cell — close to what the in-game CanvasScaler ends up with
 * once 1280-px reference design coords are mapped to actual screen pixels.
 */
const NAMES_AND_SIZES: [string, [number, number]][] = [
  ['Dachshund',   [114, 70]],   // A
  ['Tabby Cat',   [88,  88]],   // B
  ['Hedgehog',    [100, 70]],   // C
  ['Turtle',      [96,  62]],   // D
  ['Husky',       [92,  90]],   // E
  ['Siamese Cat', [84,  94]],   // F
  ['Cockatiel',   [74,  84]],   // G
  ['Ferret',      [113, 74]],   // H
  ['Chameleon',   [91,  62]],   // I
  ['Pug',         [93,  72]],   // J
  ['Rabbit',      [79,  90]],   // K
];
/**
 * Scale applied on top of the per-pet design-pixel sizes from `interestSizeSettings`.
 * The bundle's CanvasScaler reference resolution is 1280×960; we scale up so pets
 * read clearly while keeping the road visible around them — roughly the proportion
 * field33.png shows. 1.6× puts the largest pet (Dachshund 114×70 → 182×112) at
 * ~73 % of cell width.
 */
const PET_SCALE = 1.8;

export const ANIMALS: (AnimalEntry & { petWidth: number; petHeight: number })[] =
  NAMES_AND_SIZES.map(([name, [w, h]], i) => ({
    index: i,
    letter: String.fromCharCode(65 + i),
    homeLetter: String.fromCharCode(97 + i),
    name,
    petSprite: `${import.meta.env.BASE_URL}sprites/pets/${String.fromCharCode(65 + i)}.svg`,
    houseSprite: `${import.meta.env.BASE_URL}sprites/houses/${String.fromCharCode(65 + i)}.png`,
    petWidth: w * PET_SCALE,
    petHeight: h * PET_SCALE,
  }));

/** Cell size in the bundled GridLayoutGroup (m_CellSize). */
export const GAME_CELL_PX = 250;
/**
 * Houses are tall portraits (texture aspect 205×256). With `preserveAspectRatio="meet"`
 * a 192×192 box renders the house at 154×192 — wide enough to fill ~62 % of cell
 * width while reaching cell-top and cell-bottom, matching the in-game look.
 */
export const HOUSE_SIZE_PX = 180;
export const HOUSE_Y_OFFSET = -40;
/** Car (top-down view) sized like a wide pet, slightly smaller than a house. */
export const CAR_SIZE_PX = 180;
/** Pets are typically rendered slightly above cell-centre in-game so their
 *  "feet" sit near the road midline; tweak to shift the pet vertically. */
export const PET_Y_OFFSET = -25;

export const CAR_SPRITES = {
  up: `${import.meta.env.BASE_URL}sprites/car/up.svg`,
  down: `${import.meta.env.BASE_URL}sprites/car/down.svg`,
  left: `${import.meta.env.BASE_URL}sprites/car/left.svg`,
  right: `${import.meta.env.BASE_URL}sprites/car/right.svg`,
};

/** Returns the URL of the road tile sprite for the given UDLR connectivity bitmask
 *  (U=8, D=4, L=2, R=1). Returns null for an isolated cell (mask=0). */
export function roadSpriteUrl(mask: number): string | null {
  if (mask === 0) return null;
  return `${import.meta.env.BASE_URL}sprites/roads/${mask}.svg`;
}

export const BOARD_BACKGROUND = `${import.meta.env.BASE_URL}sprites/board/background.png`;

/** Pretty hue (degrees) per letter — used by the Board for halos when a pair has no sprite. */
export const ANIMAL_HUES: number[] = [
  18,   // A Dachshund: warm brown
  35,   // B Tabby: orange-brown
  280,  // C Hedgehog: purple
  130,  // D Turtle: green
  210,  // E Husky: blue-grey
  50,   // F Siamese: cream
  340,  // G Cockatiel: pink
  20,   // H Ferret: tan
  150,  // I Chameleon: green-cyan
  30,   // J Pug: tan-orange
  300,  // K Rabbit: pink-purple
];
