# Kame House — Diorama con Ray Tracing

Diorama inspirado en **Kame House**, construido con cubos texturizados y renderizado completamente por CPU. El proyecto implementa un ray tracer propio en Rust, con iluminación dinámica, materiales, reflexión, refracción, mapas normales, emisión, cielo procedural y terreno generado proceduralmente.

## Video de demostración

<video src="docs/kame_house.mp4" controls width="720">
  Tu navegador no puede reproducir el video. [Ver o descargar el video](docs/kame_house.mp4).
</video>

[Ver o descargar el video del diorama](docs/kame_house.mp4)

## Vista previa

![Vista previa del diorama](Adobe%20Express%20-%20Kame%20House%202x.gif)

## Características implementadas

- Escena de bloques inspirada en Minecraft con **23,263 bloques** y múltiples formas de cubo.
- Renderizado por ray tracing sobre CPU, sin utilizar la tarjeta de video.
- Intersección de rayos contra cuboides y aceleración espacial mediante un **BVH**.
- Renderizado paralelo por filas usando `std::thread` y el paralelismo disponible del procesador.
- Renderizado progresivo: durante el movimiento se prioriza la respuesta y, al detenerse, aumenta la calidad.
- Cámara orbital con rotación horizontal y vertical, zoom y límite de acercamiento para evitar atravesar la escena.
- Ciclo de día y noche con sol, luna, estrellas, sombras y niebla atmosférica.
- Agua con ondas, reflexión y refracción contextualizadas en el océano alrededor de la isla.
- Materiales con textura propia y parámetros independientes de albedo, especularidad, transparencia, reflectividad, índice de refracción, emisión, rugosidad y fuerza del mapa normal.
- Mapas normales calculados a partir de la información de luminancia de las texturas.
- Material emisivo para la iluminación cálida del farol de Kame House durante la noche.
- Cielo procedural tipo skybox y modo opcional de cielo panorámico con imágenes.
- Generación procedural de un área de terreno submarino de **16 × 16 cubos**.

## Controles

| Acción | Control |
|---|---|
| Rotar la cámara | Arrastrar con el botón izquierdo del mouse o `A` / `D` |
| Cambiar la altura de la cámara | Flechas `↑` / `↓` |
| Acercar o alejar | Rueda del mouse o `W` / `S` |
| Activar/desactivar rotación automática | `Espacio` |
| Avanzar o retroceder el ciclo día/noche | `Q` / `E` |
| Activar/desactivar ciclo automático | `T` |
| Cambiar al cielo panorámico | `I` |
| Cambiar música | `B` / `N` |
| Restablecer cámara y hora | `R` |
| Salir | Cerrar la ventana |

## Requisitos

- Rust y Cargo con soporte para la edición 2024.
- Una computadora con capacidad suficiente para renderizado por CPU.
- No se requiere una GPU para el ray tracing.

La ventana y la entrada de usuario utilizan `raylib`; el trazado de rayos, la geometría, los materiales, la iluminación y la aceleración BVH están implementados en el código del proyecto.

## Ejecución

Desde la raíz del proyecto:

```bash
cargo run --release
```

Para generar una imagen fija sin abrir la ventana:

```bash
cargo run --release -- --offline
```

Para generar las imágenes de una secuencia de video:

```bash
cargo run --release -- --video-frames
```

La imagen fija se guarda como `kame_house_preview.ppm` y los fotogramas como archivos `.ppm` dentro de `video_frames/`.

## Organización del código

- `src/main.rs`: ventana, controles, cámara, ciclo día/noche y modos de ejecución.
- `src/renderer.rs`: trazado de rayos, iluminación, sombras, reflexión, refracción, skybox y renderizado paralelo.
- `src/scene.rs`: carga de texturas, definición de superficies, escena, agua, cielo panorámico y normales.
- `src/geometry.rs`: rayos, cuboides, intersecciones y BVH.
- `src/generated_map.rs`: mapa comprimido, paleta de bloques y dimensiones de la escena.
- `assets/textures/`: texturas de los bloques y materiales.
- `assets/idol_sky/`: imágenes usadas por el modo de cielo panorámico.

