/**
 * The skin and cape pixels the fixture serves. Data URLs rather than the https
 * textures a real account carries: a mock that needs the network to render a
 * page is a mock that fails on a plane, and the wire allows either — a
 * `Skin`'s texture is an https URL *or* a data URL for a library blob.
 *
 * Each is a valid PNG in the classic layout (64x64; the cape 64x32) — crude,
 * but enough for the preview and the thumbnail renderer to read.
 */
export const textures = {
  classic:
    'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAAlklEQVR42u3asQmAMBRF0czhCIILiFg4hgu4iSNYO41rRbAXCwn6ybnwCtsDSSFJ6aGxbXLJpb8HAAAAAAAAVAxwrFMuOQBRAIZ5u3b3DcARAAAAQESArl/ym6V9//cAAAAQGqD4JQsAAAAAAAAAAAAAQKUAkiRJkiSpvj7/IQIAAIDQANU/kgIAAAAAAAAAAAAAICbACfnP88NnFevHAAAAAElFTkSuQmCC',
  slim: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAAk0lEQVR42u3asQmAMBRF0czhCM4QLBwjAzlCJnEG14pgLxYS9JNz4RW2B5JCktJDyzy1nkt/DwAAAAAAABgY4NjW1nMAogDkUq/dfQNwBAAAABARIOfS3qyW/dcDAABAbIDulywAAAAAAAAAAAAAAIMCSJIkSZKk8fr6hwgAAABiAwz/SAoAAAAAAAAAAAAAgJgAJ68Iw9L8Sei9AAAAAElFTkSuQmCC',
  library:
    'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAAmklEQVR42u3asQmEMBiG4cxhYX0IbiAWVziEkziBIziSA7hQhOsPCwn6k+eFr7B9ICkkKV00dk0uufT2AAAAAAAAgIoB9vWbSw5AFIBh3n779w3AEQAAAEBEgKlv850dy+fVAwAAQGyA4pcsAAAAAAAAAAAAAACVAkiSJEmSpPp6+ocIAAAAYgNU/0gKAAAAAAAAAAAAAICYACcK7N3DWX/UxQAAAABJRU5ErkJggg==',
  cape: 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAAAgCAYAAACinX6EAAAAR0lEQVR42u3QMQ0AIBAEQWRQEUrUoQTPYIEv4WeTE3BTVhv7ZrP20MorAQAAAAAAAAAAAAAAAAAAAAkBose+A5AkSZIkKV8HuwyCrdYxmrgAAAAASUVORK5CYII=',
} as const;
