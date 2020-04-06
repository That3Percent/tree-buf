/*use crate::prelude::*;
struct RLE<'a, T> {
    // TODO: (Performance) Do not require the allocation of this Vec
    sub_compressors: Vec<Box<dyn Compressor<'a, Data=T>>>,
}

impl<'a, T: 'static + PartialEq + Copy> Compressor<'a> for RLE<'a, T> {
    type Data = T;
    fn compress(&self, data: &[Self::Data], bytes: &mut Vec<u8>) -> Result<ArrayTypeId, ()> {
        // Prevent panic on indexing first item.
        if data.len() == 0 {
            return Err(());
        }
        let mut runs = Vec::new();
        let mut current_run = 1;
        let mut current_value = data[0];
        let mut values = vec![current_value];
        for item in data[1..].iter() {
            if current_value == *item {
                current_run += 1;
            } else {
                runs.push(current_run);
                current_value = *item;
                values.push(current_value);
                current_run = 1;
            }
        }

        // If no values are removed, it is determined
        // that this cannot possibly be better,
        // so don't go through the compression step
        // for nothing.
        if values.len() == data.len() {
            return Err(());
        }

        // Can't use write_with_id directly, because that would cause problems
        // with object safety.
        // See also f4aba341-af61-490f-b113-380cb4c38a77
        // 
        let type_index = bytes.len();
        bytes.push(0);
        let id = compress(&values[..], bytes, &self.sub_compressors[..]);
        bytes[type_index] = id.into();

        // TODO: FIXME: Because of the traits and such, can't compress to a 
        // a stream and re-use the existing code. So, assume PrefixVarInt for the runs for now.


        Ok(ArrayTypeId::RLE)
    }
}
*/