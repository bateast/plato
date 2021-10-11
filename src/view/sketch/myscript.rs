// https://swaggerui.myscript.com/

use sha2::Sha512;
use hmac::{Hmac, NewMac, Mac};
use hex;
use uuid::Uuid;
use serde::{Serialize};
use serde_json::Result;

#[derive(Serialize)]
struct TextConfiguration {}

#[derive(Serialize)]
enum PointerType {Pen, Touch, Eraser}
// Representation of a stroke, that is the capture of an user writing input between the moment when the writing device touches the writing surface and the moment when it is lifted from the surface. See https://developer.myscript.com/docs/interactive-ink/latest/web/myscriptjs/editing/ for information about the components of a stroke
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stroke {
    id: String, // n optional id for the stroke
    x: Vec<u32>, // the list of x coordinates of the stroke[...]
    y: Vec<u32>, // the list of y coordinates of the stroke[...]
    t: Vec<i64>, //	The list of timestamps of the stroke[...]
    // p: Vec<f32>, //	The list of pressure information of the stroke[...]
    pointer_type: PointerType, // The pointer type for the strokeEnum:
    pointer_id: 	i32, // The pointer id
}
impl Default for Stroke {
    fn default() -> Self {
        Stroke {
            id: Uuid::new_v4().to_string(),
            x: Vec::new(),
            y: Vec::new(),
            t: Vec::new(),
            // p: Vec::new(),
            pointer_type: PointerType::Pen,
            pointer_id: 0,
        }
    }
}

// a list of strokes that share the same pen style
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StrokeGroups {
    strokes: Vec<Stroke>,
    pen_style:	String, // CSS style for the pen.
    pen_style_classes:	String,// CSS classes for the pen. Classes are to be provided in the general CSS theme.
}
impl Default for StrokeGroups {
    fn default() -> Self{
        StrokeGroups {
            strokes: Vec::new(),
            pen_style: "".to_string(),
            pen_style_classes: "".to_string(),
        }
    }

}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Configuration {
    always_connected: bool,
    lang:	String, //lang  example: en_US
    // math:	&'a MathConfiguration,
    text:	TextConfiguration,
    // export	ExportConfiguration{...}
    // diagram	DiagramConfiguration{...}
    // gesture	GestureConfiguration{...}
    // raw-content	RawContentConfiguration{...}
}
impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            always_connected: false,
            lang: "en_US".to_string(),
            text: TextConfiguration{},
        }
    }
}

#[derive(Serialize)]
enum ContentType { Text } //, Math, Diagram, RawContent, TextDocument }
#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ConversionState { DigitalPublish, DigitalEdit }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchInput {
    width: u32, // title: width of the writing area
    height:	u32, // height of the writing area
    content_type: ContentType, // recognition typeEnum: [ Text, Math, Diagram, Raw Content, Text Document ]
    conversion_state: ConversionState, //  target of conversion, no conversion will be made if that parameter is not provided
    theme: String, // A global CSS styling for your content.
    stroke_groups: StrokeGroups, //
    configuration: Configuration, //
    x_dpi: f32, // x resolution of the writing area in dpi
    y_dpi: f32, // y resolution of the writing area in dpi
}

type HmacSha512 = Hmac<Sha512>;
fn compute_hmac(application_key: String, hmac_key: String, json_input: String) -> String {
    let mut key = String::new();
    key.push_str(&application_key);
    key.push_str(&hmac_key);

    let mut mac = HmacSha512::new_from_slice(&key.into_bytes())
        .expect("HMAC can take key of any size");
    mac.update (&json_input.into_bytes());

    let final_mac = mac.finalize();
    hex::encode(final_mac.into_bytes())
}

fn create_json_request(batch: &BatchInput) -> Result<String> {
    let j = serde_json::to_string(batch)?;
    Ok(j)
}
