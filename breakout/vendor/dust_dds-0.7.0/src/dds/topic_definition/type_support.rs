use std::io::Read;

use crate::{
    implementation::{
        data_representation_builtin_endpoints::parameter_id_values::PID_SENTINEL,
        payload_serializer_deserializer::{
            cdr_deserializer::ClassicCdrDeserializer, cdr_serializer::ClassicCdrSerializer,
            endianness::CdrEndianness, parameter_list_deserializer::ParameterListCdrDeserializer,
            parameter_list_serializer::ParameterListCdrSerializer,
        },
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
    },
    serialized_payload::{
        cdr::{deserialize::CdrDeserialize, serialize::CdrSerialize},
        parameter_list::{
            deserialize::ParameterListDeserialize, serialize::ParameterListSerialize,
            serializer::ParameterListSerializer,
        },
    },
};

pub use dust_dds_derive::{DdsDeserialize, DdsHasKey, DdsSerialize, DdsTypeXml};

#[doc(hidden)]
pub trait DynamicTypeInterface {
    fn has_key(&self) -> bool;

    fn get_serialized_key_from_serialized_foo(&self, serialized_foo: &[u8]) -> DdsResult<Vec<u8>>;

    fn instance_handle_from_serialized_foo(
        &self,
        serialized_foo: &[u8],
    ) -> DdsResult<InstanceHandle>;

    fn instance_handle_from_serialized_key(
        &self,
        serialized_key: &[u8],
    ) -> DdsResult<InstanceHandle>;

    fn xml_type(&self) -> String;
}

/// This trait indicates whether the associated type is keyed or not, i.e. if the middleware
/// should manage different instances of the type.
///
/// ## Derivable
///
/// This trait can be automatically derived. If the struct has any field marked `#[dust_dds(key)]`
/// then HAS_KEY will be set to 'true' otherwise will be set to 'false'.
pub trait DdsHasKey {
    const HAS_KEY: bool;
}

/// This trait defines how to serialize the information contained in a data structure to be published.
///
/// The information generated by the method of this trait is typically visible on the
/// `serializedData` element of the Data submessage when transmitting a published sample.
///
/// ## Derivable
///
/// This trait can be automatically derived if the struct implements either `CdrSerialize` or `ParameterListSerialize`.
/// The format to be used for serializing can be selected by applying the '#[dust_dds(format = ...)]' attribute to the container.
/// Available format options are "CDR_LE", "CDR_BE", "PL_CDR_LE" and "PL_CDR_BE".
pub trait DdsSerialize {
    fn serialize_data(&self, writer: impl std::io::Write) -> DdsResult<()>;
}

/// This trait describes how the bytes can be deserialize to construct the data structure.
///
/// This trait is typically used when reading the data from the samples from the DataReader.
/// The `'de` lifetime of this trait is the lifetime of data that may be borrowed from the input when deserializing.
///
/// ## Derivable
///
/// This trait can be automatically derived if the struct implements either `CdrSerialize` or `ParameterListSerialize`.
/// The format to be used for deserializing can be selected by applying the '#[dust_dds(format = ...)]' attribute to the container.
/// Available format options are "CDR_LE", "CDR_BE", "PL_CDR_LE" and "PL_CDR_BE".
pub trait DdsDeserialize<'de>: Sized {
    fn deserialize_data(serialized_data: &'de [u8]) -> DdsResult<Self>;
}

/// This trait defines the key associated with the type. The key is used to identify different instances of the type.
/// The returned key object must implement ['CdrSerialize'] and ['CdrDeserialize'] since CDR is the format always
/// used to transmit the key information on the wire and this can not be modified by the user.
///
/// ## Derivable
///
/// This trait can be automatically derived if all the field marked `#[dust_dds(key)]` implement ['CdrSerialize'] and ['CdrDeserialize']
///
pub trait DdsKey {
    type Key: CdrSerialize + for<'de> CdrDeserialize<'de>;

    fn get_key(&self) -> DdsResult<Self::Key>;

    fn get_key_from_serialized_data(serialized_foo: &[u8]) -> DdsResult<Self::Key>;
}

/// This trait defines the optional type representation for a user type. The type representation
/// returned by the function in this trait must follow the description in 7.3.2 XML Type Representation
/// of the [OMG DDS-XTypes standard](https://www.omg.org/spec/DDS-XTypes/1.3/).
///
/// ## Derivable
///
/// This trait can be automatically derived for every DustDDS supported type.
pub trait DdsTypeXml {
    fn get_type_xml() -> Option<String>;
}

/// This is a convenience derive to allow the user to easily derive all the different traits needed for a type to be used for
/// communication with DustDDS. If the individual traits are manually derived then this derive should not be used.
///
/// This trait can be automatically derived. The generated trait uses by default a CdrLe
/// representation and it determines whether the type is keyed or not depending on whether
/// any field is marked `#[dust_dds(key)]` or not.
///
/// An example of a typical usage of derive is the following:
///
/// ```rust
///     use dust_dds::topic_definition::type_support::DdsType;
///
///     #[derive(DdsType)]
///     struct KeyedData {
///         #[dust_dds(key)]
///         id: u8,
///         value: u32,
///     }
/// ```
///
/// It is also possible to derive structs with a lifetime:
///
/// ```rust
///     use dust_dds::topic_definition::type_support::DdsType;
///     use std::borrow::Cow;
///
///     #[derive(DdsType)]
///     struct BorrowedData<'a> {
///         #[dust_dds(key)]
///         id: u8,
///         value: &'a [u8],
///     }
/// ```
///
pub use dust_dds_derive::DdsType;

type RepresentationIdentifier = [u8; 2];
type RepresentationOptions = [u8; 2];

const CDR_BE: RepresentationIdentifier = [0x00, 0x00];
const CDR_LE: RepresentationIdentifier = [0x00, 0x01];
const PL_CDR_BE: RepresentationIdentifier = [0x00, 0x02];
const PL_CDR_LE: RepresentationIdentifier = [0x00, 0x03];
const REPRESENTATION_OPTIONS: RepresentationOptions = [0x00, 0x00];

/// This is a helper function to serialize a type implementing [`CdrSerialize`] using the RTPS defined classic CDR representation with LittleEndian endianness.
pub fn serialize_rtps_classic_cdr_le(
    value: &impl CdrSerialize,
    mut writer: impl std::io::Write,
) -> DdsResult<()> {
    writer.write_all(&CDR_LE)?;
    writer.write_all(&REPRESENTATION_OPTIONS)?;
    let mut serializer = ClassicCdrSerializer::new(writer, CdrEndianness::LittleEndian);
    CdrSerialize::serialize(value, &mut serializer)?;
    Ok(())
}

/// This is a helper function to serialize a type implementing [`CdrSerialize`] using the RTPS defined classic CDR representation with BigEndian endianness.
pub fn serialize_rtps_classic_cdr_be(
    value: &impl CdrSerialize,
    mut writer: impl std::io::Write,
) -> DdsResult<()> {
    writer.write_all(&CDR_BE)?;
    writer.write_all(&REPRESENTATION_OPTIONS)?;
    let mut serializer = ClassicCdrSerializer::new(writer, CdrEndianness::BigEndian);
    CdrSerialize::serialize(value, &mut serializer)?;
    Ok(())
}

/// This is a helper function to serialize a type implementing [`ParameterListSerialize`] using the RTPS defined CDR Parameter List representation with Little Endian endianness
pub fn serialize_rtps_cdr_pl_le(
    value: &impl ParameterListSerialize,
    mut writer: impl std::io::Write,
) -> DdsResult<()> {
    writer.write_all(&PL_CDR_LE)?;
    writer.write_all(&REPRESENTATION_OPTIONS)?;
    let mut serializer = ParameterListCdrSerializer::new(writer, CdrEndianness::LittleEndian);
    ParameterListSerialize::serialize(value, &mut serializer)?;
    serializer.write(PID_SENTINEL, &())?;
    Ok(())
}

/// This is a helper function to serialize a type implementing [`ParameterListSerialize`] using the RTPS defined CDR Parameter List representation with Big Endian endianness
pub fn serialize_rtps_cdr_pl_be(
    value: &impl ParameterListSerialize,
    mut writer: impl std::io::Write,
) -> DdsResult<()> {
    writer.write_all(&PL_CDR_BE)?;
    writer.write_all(&REPRESENTATION_OPTIONS)?;
    let mut serializer = ParameterListCdrSerializer::new(writer, CdrEndianness::BigEndian);
    ParameterListSerialize::serialize(value, &mut serializer)?;
    serializer.write(PID_SENTINEL, &())?;
    Ok(())
}

/// This is a helper function to deserialize a type implementing both [`CdrDeserialize`] and [`ParameterListDeserialize`] using either
/// the RTPS classic CDR or Parameter List representation.
/// The representation to be used is automatically determined from the representation identifier and options
pub fn deserialize_rtps<'de, T>(serialized_data: &mut &'de [u8]) -> DdsResult<T>
where
    T: CdrDeserialize<'de> + ParameterListDeserialize<'de>,
{
    let mut representation_identifier = [0u8, 0];
    serialized_data
        .read_exact(&mut representation_identifier)
        .map_err(|err| DdsError::Error(err.to_string()))?;

    let mut representation_option = [0u8, 0];
    serialized_data
        .read_exact(&mut representation_option)
        .map_err(|err| DdsError::Error(err.to_string()))?;

    let value = match representation_identifier {
        CDR_BE => {
            let mut deserializer =
                ClassicCdrDeserializer::new(serialized_data, CdrEndianness::BigEndian);
            Ok(CdrDeserialize::deserialize(&mut deserializer)?)
        }
        CDR_LE => {
            let mut deserializer =
                ClassicCdrDeserializer::new(serialized_data, CdrEndianness::LittleEndian);
            Ok(CdrDeserialize::deserialize(&mut deserializer)?)
        }
        PL_CDR_BE => {
            let mut deserializer =
                ParameterListCdrDeserializer::new(serialized_data, CdrEndianness::BigEndian);
            Ok(ParameterListDeserialize::deserialize(&mut deserializer)?)
        }
        PL_CDR_LE => {
            let mut deserializer =
                ParameterListCdrDeserializer::new(serialized_data, CdrEndianness::LittleEndian);
            Ok(ParameterListDeserialize::deserialize(&mut deserializer)?)
        }
        _ => Err(DdsError::Error(
            "Unknownn representation identifier".to_string(),
        )),
    }?;
    Ok(value)
}

/// This is a helper function to deserialize a type implementing [`CdrDeserialize`] using the RTPS classic CDR representation.
/// The representation endianness to be used is automatically determined from the representation identifier and options
pub fn deserialize_rtps_classic_cdr<'de, T>(serialized_data: &mut &'de [u8]) -> DdsResult<T>
where
    T: CdrDeserialize<'de>,
{
    let mut representation_identifier = [0u8, 0];
    serialized_data
        .read_exact(&mut representation_identifier)
        .map_err(|err| DdsError::Error(err.to_string()))?;

    let mut representation_option = [0u8, 0];
    serialized_data
        .read_exact(&mut representation_option)
        .map_err(|err| DdsError::Error(err.to_string()))?;

    let mut deserializer = match representation_identifier {
        CDR_BE => Ok(ClassicCdrDeserializer::new(
            serialized_data,
            CdrEndianness::BigEndian,
        )),
        CDR_LE => Ok(ClassicCdrDeserializer::new(
            serialized_data,
            CdrEndianness::LittleEndian,
        )),
        _ => Err(DdsError::Error(
            "Unknownn representation identifier".to_string(),
        )),
    }?;
    let value = CdrDeserialize::deserialize(&mut deserializer)?;
    Ok(value)
}

/// This is a helper function to deserialize a type implementing [`ParameterListDeserialize`] using the RTPS Parameter List representation.
/// The representation endianness to be used is automatically determined from the representation identifier and options
pub fn deserialize_rtps_cdr_pl<'de, T>(serialized_data: &mut &'de [u8]) -> DdsResult<T>
where
    T: ParameterListDeserialize<'de>,
{
    let mut representation_identifier = [0u8, 0];
    serialized_data
        .read_exact(&mut representation_identifier)
        .map_err(|err| DdsError::Error(err.to_string()))?;

    let mut representation_option = [0u8, 0];
    serialized_data
        .read_exact(&mut representation_option)
        .map_err(|err| DdsError::Error(err.to_string()))?;

    let mut deserializer = match representation_identifier {
        PL_CDR_BE => Ok(ParameterListCdrDeserializer::new(
            serialized_data,
            CdrEndianness::BigEndian,
        )),
        PL_CDR_LE => Ok(ParameterListCdrDeserializer::new(
            serialized_data,
            CdrEndianness::LittleEndian,
        )),
        _ => Err(DdsError::Error(
            "Unknownn representation identifier".to_string(),
        )),
    }?;
    let value = ParameterListDeserialize::deserialize(&mut deserializer)?;
    Ok(value)
}
