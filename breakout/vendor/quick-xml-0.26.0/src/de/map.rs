//! Serde `Deserializer` module

use crate::{
    de::escape::EscapedDeserializer,
    de::seq::{not_in, TagFilter},
    de::simple_type::SimpleTypeDeserializer,
    de::{str2bool, DeEvent, Deserializer, XmlRead, INNER_VALUE, UNFLATTEN_PREFIX},
    errors::serialize::DeError,
    events::attributes::IterState,
    events::BytesStart,
};
use serde::de::{self, DeserializeSeed, IntoDeserializer, SeqAccess, Visitor};
use serde::serde_if_integer128;
use std::borrow::Cow;
use std::ops::Range;

/// Defines a source that should be used to deserialize a value in the next call
/// to [`next_value_seed()`](de::MapAccess::next_value_seed)
#[derive(Debug, PartialEq)]
enum ValueSource {
    /// Source are not specified, because [`next_key_seed()`] not yet called.
    /// This is an initial state and state after deserializing value
    /// (after call of [`next_value_seed()`]).
    ///
    /// Attempt to call [`next_value_seed()`] while accessor in this state would
    /// return a [`DeError::KeyNotRead`] error.
    ///
    /// [`next_key_seed()`]: de::MapAccess::next_key_seed
    /// [`next_value_seed()`]: de::MapAccess::next_value_seed
    Unknown,
    /// Next value should be deserialized from an attribute value; value is located
    /// at specified span.
    Attribute(Range<usize>),
    /// Value should be deserialized from the text content of the XML node, which
    /// represented or by an ordinary text node, or by a CDATA node:
    ///
    /// ```xml
    /// <any-tag>
    ///     <key>text content</key>
    /// <!--     ^^^^^^^^^^^^ - this will be used to deserialize map value -->
    /// </any-tag>
    /// ```
    /// ```xml
    /// <any-tag>
    ///     <key><![CDATA[cdata content]]></key>
    /// <!--              ^^^^^^^^^^^^^ - this will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    Text,
    /// Next value should be deserialized from an element with an any name, except
    /// elements with a name matching one of the struct fields. Corresponding tag
    /// name will always be associated with a field with name [`INNER_VALUE`].
    ///
    /// That state is set when call to [`peek()`] returns a [`Start`] event, which
    /// [`name()`] is not listed in the [list of known fields] (which for a struct
    /// is a list of field names, and for a map that is an empty list), _and_
    /// struct has a field with a special name [`INNER_VALUE`].
    ///
    /// When in this state, next event, returned by [`next()`], will be a [`Start`],
    /// which represents both a key, and a value. Value would be deserialized from
    /// the whole element and how is will be done determined by the value deserializer.
    /// The [`MapAccess`] do not consume any events in that state.
    ///
    /// Because in that state any encountered `<tag>` is mapped to the [`INNER_VALUE`]
    /// field, it is possible to use tag name as an enum discriminator, so `enum`s
    /// can be deserialized from that XMLs:
    ///
    /// ```xml
    /// <any-tag>
    ///     <variant1>...</variant1>
    /// <!-- ~~~~~~~~               - this data will determine that this is Enum::variant1 -->
    /// <!--^^^^^^^^^^^^^^^^^^^^^^^ - this data will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    /// ```xml
    /// <any-tag>
    ///     <variant2>...</variant2>
    /// <!-- ~~~~~~~~               - this data will determine that this is Enum::variant2 -->
    /// <!--^^^^^^^^^^^^^^^^^^^^^^^ - this data will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    ///
    /// both can be deserialized into
    ///
    /// ```ignore
    /// enum Enum {
    ///   variant1,
    ///   variant2,
    /// }
    /// struct AnyName {
    ///   #[serde(rename = "$value")]
    ///   field: Enum,
    /// }
    /// ```
    ///
    /// That is possible, because value deserializer have access to the full content
    /// of a `<variant1>...</variant1>` or `<variant2>...</variant2>` node, including
    /// the tag name.
    ///
    /// Currently, processing of that enum variant is fully equivalent to the
    /// processing of a [`Text`] variant. Split of variants made for clarity.
    ///
    /// [`Start`]: DeEvent::Start
    /// [`peek()`]: Deserializer::peek()
    /// [`next()`]: Deserializer::next()
    /// [`name()`]: BytesStart::name()
    /// [`Text`]: Self::Text
    /// [list of known fields]: MapAccess::fields
    Content,
    /// Next value should be deserialized from an element with a dedicated name.
    /// If deserialized type is a sequence, then that sequence will collect all
    /// elements with the same name until it will be filled. If not all elements
    /// would be consumed, the rest will be ignored.
    ///
    /// That state is set when call to [`peek()`] returns a [`Start`] event, which
    /// [`name()`] represents a field name. That name will be deserialized as a key.
    ///
    /// When in this state, next event, returned by [`next()`], will be a [`Start`],
    /// which represents both a key, and a value. Value would be deserialized from
    /// the whole element and how is will be done determined by the value deserializer.
    /// The [`MapAccess`] do not consume any events in that state.
    ///
    /// An illustration below shows, what data is used to deserialize key and value:
    /// ```xml
    /// <any-tag>
    ///     <key>...</key>
    /// <!-- ~~~           - this data will be used to deserialize a map key -->
    /// <!--^^^^^^^^^^^^^^ - this data will be used to deserialize a map value -->
    /// </any-tag>
    /// ```
    ///
    /// Although value deserializer will have access to the full content of a `<key>`
    /// node (including the tag name), it will not get much benefits from that,
    /// because tag name will always be fixed for a given map field (equal to a
    /// field name). So, if the field type is an `enum`, it cannot select its
    /// variant based on the tag name. If that is needed, then [`Content`] variant
    /// of this enum should be used. Such usage is enabled by annotating a struct
    /// field as "content" field, which implemented as given the field a special
    /// [`INNER_VALUE`] name.
    ///
    /// [`Start`]: DeEvent::Start
    /// [`peek()`]: Deserializer::peek()
    /// [`next()`]: Deserializer::next()
    /// [`name()`]: BytesStart::name()
    /// [`Content`]: Self::Content
    Nested,
}

/// A deserializer that extracts map-like structures from an XML. This deserializer
/// represents a one XML tag:
///
/// ```xml
/// <tag>...</tag>
/// ```
///
/// Name of this tag is stored in a [`Self::start`] property.
///
/// # Lifetimes
///
/// - `'de` lifetime represents a buffer, from which deserialized values can
///   borrow their data. Depending on the underlying reader, there can be an
///   internal buffer of deserializer (i.e. deserializer itself) or an input
///   (in that case it is possible to approach zero-copy deserialization).
///
/// - `'a` lifetime represents a parent deserializer, which could own the data
///   buffer.
pub(crate) struct MapAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Tag -- owner of attributes
    start: BytesStart<'de>,
    de: &'a mut Deserializer<'de, R>,
    /// State of the iterator over attributes. Contains the next position in the
    /// inner `start` slice, from which next attribute should be parsed.
    iter: IterState,
    /// Current state of the accessor that determines what next call to API
    /// methods should return.
    source: ValueSource,
    /// List of field names of the struct. It is empty for maps
    fields: &'static [&'static str],
    /// If `true`, then the deserialized struct has a field with a special name:
    /// [`INNER_VALUE`]. That field should be deserialized from the text content
    /// of an XML node:
    ///
    /// ```xml
    /// <tag>value for INNER_VALUE field<tag>
    /// ```
    has_value_field: bool,
    /// list of fields yet to unflatten (defined as starting with $unflatten=)
    unflatten_fields: Vec<&'static [u8]>,
}

impl<'de, 'a, R> MapAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    /// Create a new MapAccess
    pub fn new(
        de: &'a mut Deserializer<'de, R>,
        start: BytesStart<'de>,
        fields: &'static [&'static str],
    ) -> Result<Self, DeError> {
        Ok(MapAccess {
            de,
            iter: IterState::new(start.name().as_ref().len(), false),
            start,
            source: ValueSource::Unknown,
            fields,
            has_value_field: fields.contains(&INNER_VALUE),
            unflatten_fields: fields
                .iter()
                .filter(|f| f.starts_with(UNFLATTEN_PREFIX))
                .map(|f| f.as_bytes())
                .collect(),
        })
    }
}

impl<'de, 'a, R> de::MapAccess<'de> for MapAccess<'de, 'a, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        debug_assert_eq!(self.source, ValueSource::Unknown);

        // FIXME: There error positions counted from the start of tag name - need global position
        let slice = &self.start.buf;
        let decoder = self.de.reader.decoder();

        if let Some(a) = self.iter.next(slice).transpose()? {
            // try getting map from attributes (key= "value")
            let (key, value) = a.into();
            self.source = ValueSource::Attribute(value.unwrap_or_default());
            seed.deserialize(EscapedDeserializer::new(
                Cow::Borrowed(&slice[key]),
                decoder,
                false,
            ))
            .map(Some)
        } else {
            // try getting from events (<key>value</key>)
            match self.de.peek()? {
                DeEvent::Text(_) | DeEvent::CData(_) => {
                    self.source = ValueSource::Text;
                    // Deserialize `key` from special attribute name which means
                    // that value should be taken from the text content of the
                    // XML node
                    seed.deserialize(INNER_VALUE.into_deserializer()).map(Some)
                }
                // Used to deserialize collections of enums, like:
                // <root>
                //   <A/>
                //   <B/>
                //   <C/>
                // </root>
                //
                // into
                //
                // enum Enum { A, B, С }
                // struct Root {
                //     #[serde(rename = "$value")]
                //     items: Vec<Enum>,
                // }
                // TODO: This should be handled by #[serde(flatten)]
                // See https://github.com/serde-rs/serde/issues/1905
                DeEvent::Start(e) if self.has_value_field && not_in(self.fields, e, decoder)? => {
                    self.source = ValueSource::Content;
                    seed.deserialize(INNER_VALUE.into_deserializer()).map(Some)
                }
                DeEvent::Start(e) => {
                    self.source = ValueSource::Nested;
                    let key = if let Some(p) = self
                        .unflatten_fields
                        .iter()
                        .position(|f| e.name().as_ref() == &f[UNFLATTEN_PREFIX.len()..])
                    {
                        // Used to deserialize elements, like:
                        // <root>
                        //   <xxx>test</xxx>
                        // </root>
                        //
                        // into
                        //
                        // struct Root {
                        //     #[serde(rename = "$unflatten=xxx")]
                        //     xxx: String,
                        // }
                        seed.deserialize(self.unflatten_fields.remove(p).into_deserializer())
                    } else {
                        let name = Cow::Borrowed(e.local_name().into_inner());
                        seed.deserialize(EscapedDeserializer::new(name, decoder, false))
                    };
                    key.map(Some)
                }
                // Stop iteration after reaching a closing tag
                DeEvent::End(e) if e.name() == self.start.name() => Ok(None),
                // This is a unmatched closing tag, so the XML is invalid
                DeEvent::End(e) => Err(DeError::UnexpectedEnd(e.name().as_ref().to_owned())),
                // We cannot get `Eof` legally, because we always inside of the
                // opened tag `self.start`
                DeEvent::Eof => Err(DeError::UnexpectedEof),
            }
        }
    }

    fn next_value_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<K::Value, Self::Error> {
        match std::mem::replace(&mut self.source, ValueSource::Unknown) {
            ValueSource::Attribute(value) => seed.deserialize(SimpleTypeDeserializer::from_part(
                &self.start.buf,
                value,
                true,
                self.de.reader.decoder(),
            )),
            // This arm processes the following XML shape:
            // <any-tag>
            //   text value
            // </any-tag>
            // The whole map represented by an `<any-tag>` element, the map key
            // is implicit and equals to the `INNER_VALUE` constant, and the value
            // is a `Text` or a `CData` event (the value deserializer will see one
            // of that events)
            // This case are checked by "xml_schema_lists::element" tests in tests/serde-de.rs
            ValueSource::Text => match self.de.next()? {
                DeEvent::Text(e) => seed.deserialize(SimpleTypeDeserializer::from_cow(
                    e.into_inner(),
                    true,
                    self.de.reader.decoder(),
                )),
                DeEvent::CData(e) => seed.deserialize(SimpleTypeDeserializer::from_cow(
                    e.into_inner(),
                    false,
                    self.de.reader.decoder(),
                )),
                // SAFETY: We set `Text` only when we seen `Text` or `CData`
                _ => unreachable!(),
            },
            // This arm processes the following XML shape:
            // <any-tag>
            //   <any>...</any>
            // </any-tag>
            // The whole map represented by an `<any-tag>` element, the map key
            // is implicit and equals to the `INNER_VALUE` constant, and the value
            // is a `Start` event (the value deserializer will see that event)
            ValueSource::Content => seed.deserialize(MapValueDeserializer {
                map: self,
                allow_start: false,
            }),
            // This arm processes the following XML shape:
            // <any-tag>
            //   <tag>...</tag>
            // </any-tag>
            // The whole map represented by an `<any-tag>` element, the map key
            // is a `tag`, and the value is a `Start` event (the value deserializer
            // will see that event)
            ValueSource::Nested => seed.deserialize(MapValueDeserializer {
                map: self,
                allow_start: true,
            }),
            ValueSource::Unknown => Err(DeError::KeyNotRead),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! forward {
    (
        $deserialize:ident
        $(
            ($($name:ident : $type:ty),*)
        )?
    ) => {
        #[inline]
        fn $deserialize<V: Visitor<'de>>(
            self,
            $($($name: $type,)*)?
            visitor: V
        ) -> Result<V::Value, Self::Error> {
            self.map.de.$deserialize($($($name,)*)? visitor)
        }
    };
}

/// A deserializer for a value of map or struct. That deserializer slightly
/// differently processes events for a primitive types and sequences than
/// a [`Deserializer`].
struct MapValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    /// Access to the map that created this deserializer. Gives access to the
    /// context, such as list of fields, that current map known about.
    map: &'m mut MapAccess<'de, 'a, R>,
    /// Determines, should [`Deserializer::next_text_impl()`] expand the second
    /// level of tags or not.
    ///
    /// If this field is `true`, we process the following XML shape:
    ///
    /// ```xml
    /// <any-tag>
    ///   <tag>...</tag>
    /// </any-tag>
    /// ```
    ///
    /// The whole map represented by an `<any-tag>` element, the map key is a `tag`,
    /// and the value starts with is a `Start("tag")` (the value deserializer will
    /// see that event first) and extended to the matching `End("tag")` event.
    /// In order to deserialize primitives (such as `usize`) we need to allow to
    /// look inside the one levels of tags, so the
    ///
    /// ```xml
    /// <tag>42<tag>
    /// ```
    ///
    /// could be deserialized into `42usize` without problems, and at the same time
    ///
    /// ```xml
    /// <tag>
    ///   <key1/>
    ///   <key2/>
    ///   <!--...-->
    /// <tag>
    /// ```
    /// could be deserialized to a struct.
    ///
    /// If this field is `false`, we processes the one of following XML shapes:
    ///
    /// ```xml
    /// <any-tag>
    ///   text value
    /// </any-tag>
    /// ```
    /// ```xml
    /// <any-tag>
    ///   <![CDATA[cdata value]]>
    /// </any-tag>
    /// ```
    /// ```xml
    /// <any-tag>
    ///   <any>...</any>
    /// </any-tag>
    /// ```
    ///
    /// The whole map represented by an `<any-tag>` element, the map key is
    /// implicit and equals to the [`INNER_VALUE`] constant, and the value is
    /// a [`Text`], a [`CData`], or a [`Start`] event (the value deserializer
    /// will see one of those events). In the first two cases the value of this
    /// field do not matter (because we already see the textual event and there
    /// no reasons to look "inside" something), but in the last case the primitives
    /// should raise a deserialization error, because that means that you trying
    /// to deserialize the following struct:
    ///
    /// ```ignore
    /// struct AnyName {
    ///   #[serde(rename = "$value")]
    ///   any_name: String,
    /// }
    /// ```
    /// which means that `any_name` should get a content of the `<any-tag>` element.
    ///
    /// Changing this can be valuable for <https://github.com/tafia/quick-xml/issues/383>,
    /// but those fields should be explicitly marked that they want to get any
    /// possible markup as a `String` and that mark is different from marking them
    /// as accepting "text content" which the currently `$value` means.
    ///
    /// [`Text`]: DeEvent::Text
    /// [`CData`]: DeEvent::CData
    /// [`Start`]: DeEvent::Start
    allow_start: bool,
}

impl<'de, 'a, 'm, R> MapValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    /// Returns a text event, used inside [`deserialize_primitives!()`]
    #[inline]
    fn next_text(&mut self, unescape: bool) -> Result<Cow<'de, str>, DeError> {
        self.map.de.next_text_impl(unescape, self.allow_start)
    }
}

impl<'de, 'a, 'm, R> de::Deserializer<'de> for MapValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    deserialize_primitives!(mut);

    forward!(deserialize_option);
    forward!(deserialize_unit);
    forward!(deserialize_unit_struct(name: &'static str));
    forward!(deserialize_newtype_struct(name: &'static str));

    forward!(deserialize_map);
    forward!(deserialize_struct(
        name: &'static str,
        fields: &'static [&'static str]
    ));

    forward!(deserialize_enum(
        name: &'static str,
        variants: &'static [&'static str]
    ));

    forward!(deserialize_any);
    forward!(deserialize_ignored_any);

    /// Tuple representation is the same as [sequences](#method.deserialize_seq).
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    /// Named tuple representation is the same as [unnamed tuples](#method.deserialize_tuple).
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let filter = if self.allow_start {
            match self.map.de.peek()? {
                // Clone is cheap if event borrows from the input
                DeEvent::Start(e) => TagFilter::Include(e.clone()),
                // SAFETY: we use that deserializer with `allow_start == true`
                // only from the `MapAccess::next_value_seed` and only when we
                // peeked `Start` event
                _ => unreachable!(),
            }
        } else {
            TagFilter::Exclude(self.map.fields)
        };
        visitor.visit_seq(MapValueSeqAccess {
            #[cfg(feature = "overlapped-lists")]
            checkpoint: self.map.de.skip_checkpoint(),

            map: self.map,
            filter,
        })
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        self.map.de.is_human_readable()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// An accessor to sequence elements forming a value for struct field.
/// Technically, this sequence is flattened out into structure and sequence
/// elements are overlapped with other fields of a structure
struct MapValueSeqAccess<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    /// Accessor to a map that creates this accessor and to a deserializer for
    /// a sequence items.
    map: &'m mut MapAccess<'de, 'a, R>,
    /// Filter that determines whether a tag is a part of this sequence.
    ///
    /// When feature `overlapped-lists` is not activated, iteration will stop
    /// when found a tag that does not pass this filter.
    ///
    /// When feature `overlapped-lists` is activated, all tags, that not pass
    /// this check, will be skipped.
    filter: TagFilter<'de>,

    /// Checkpoint after which all skipped events should be returned. All events,
    /// that was skipped before creating this checkpoint, will still stay buffered
    /// and will not be returned
    #[cfg(feature = "overlapped-lists")]
    checkpoint: usize,
}

#[cfg(feature = "overlapped-lists")]
impl<'de, 'a, 'm, R> Drop for MapValueSeqAccess<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    fn drop(&mut self) {
        self.map.de.start_replay(self.checkpoint);
    }
}

impl<'de, 'a, 'm, R> SeqAccess<'de> for MapValueSeqAccess<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, DeError>
    where
        T: DeserializeSeed<'de>,
    {
        let decoder = self.map.de.reader.decoder();
        loop {
            break match self.map.de.peek()? {
                // If we see a tag that we not interested, skip it
                #[cfg(feature = "overlapped-lists")]
                DeEvent::Start(e) if !self.filter.is_suitable(e, decoder)? => {
                    self.map.de.skip()?;
                    continue;
                }
                // Stop iteration when list elements ends
                #[cfg(not(feature = "overlapped-lists"))]
                DeEvent::Start(e) if !self.filter.is_suitable(e, decoder)? => Ok(None),

                // Stop iteration after reaching a closing tag
                DeEvent::End(e) if e.name() == self.map.start.name() => Ok(None),
                // This is a unmatched closing tag, so the XML is invalid
                DeEvent::End(e) => Err(DeError::UnexpectedEnd(e.name().as_ref().to_owned())),
                // We cannot get `Eof` legally, because we always inside of the
                // opened tag `self.map.start`
                DeEvent::Eof => Err(DeError::UnexpectedEof),

                // Start(tag), Text, CData
                _ => seed
                    .deserialize(SeqValueDeserializer { map: self.map })
                    .map(Some),
            };
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////

/// A deserializer for a value of sequence.
struct SeqValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    /// Access to the map that created this deserializer. Gives access to the
    /// context, such as list of fields, that current map known about.
    map: &'m mut MapAccess<'de, 'a, R>,
}

impl<'de, 'a, 'm, R> SeqValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    /// Returns a text event, used inside [`deserialize_primitives!()`]
    #[inline]
    fn next_text(&mut self, unescape: bool) -> Result<Cow<'de, str>, DeError> {
        self.map.de.next_text_impl(unescape, true)
    }
}

impl<'de, 'a, 'm, R> de::Deserializer<'de> for SeqValueDeserializer<'de, 'a, 'm, R>
where
    R: XmlRead<'de>,
{
    type Error = DeError;

    deserialize_primitives!(mut);

    forward!(deserialize_option);
    forward!(deserialize_unit);
    forward!(deserialize_unit_struct(name: &'static str));
    forward!(deserialize_newtype_struct(name: &'static str));

    forward!(deserialize_map);
    forward!(deserialize_struct(
        name: &'static str,
        fields: &'static [&'static str]
    ));

    forward!(deserialize_enum(
        name: &'static str,
        variants: &'static [&'static str]
    ));

    forward!(deserialize_any);
    forward!(deserialize_ignored_any);

    /// Representation of tuples the same as [sequences](#method.deserialize_seq).
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    /// Representation of named tuples the same as [unnamed tuples](#method.deserialize_tuple).
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, DeError>
    where
        V: Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.map.de.next()? {
            DeEvent::Text(e) => SimpleTypeDeserializer::from_cow(
                // Comment to prevent auto-formatting and keep Text and Cdata similar
                e.into_inner(),
                true,
                self.map.de.reader.decoder(),
            )
            .deserialize_seq(visitor),
            DeEvent::CData(e) => SimpleTypeDeserializer::from_cow(
                e.into_inner(),
                false,
                self.map.de.reader.decoder(),
            )
            .deserialize_seq(visitor),
            // This is a sequence element. We cannot treat it as another flatten
            // sequence if type will require `deserialize_seq` We instead forward
            // it to `xs:simpleType` implementation
            DeEvent::Start(e) => {
                let value = match self.map.de.next()? {
                    DeEvent::Text(e) => SimpleTypeDeserializer::from_cow(
                        e.into_inner(),
                        true,
                        self.map.de.reader.decoder(),
                    )
                    .deserialize_seq(visitor),
                    DeEvent::CData(e) => SimpleTypeDeserializer::from_cow(
                        e.into_inner(),
                        false,
                        self.map.de.reader.decoder(),
                    )
                    .deserialize_seq(visitor),
                    e => Err(DeError::Unsupported(
                        format!("unsupported event {:?}", e).into(),
                    )),
                };
                // TODO: May be assert that here we expect only matching closing tag?
                self.map.de.read_to_end(e.name())?;
                value
            }
            // SAFETY: we use that deserializer only when Start(element), Text,
            // or CData event Start(tag), Text, CData was peeked already
            _ => unreachable!(),
        }
    }

    #[inline]
    fn is_human_readable(&self) -> bool {
        self.map.de.is_human_readable()
    }
}
