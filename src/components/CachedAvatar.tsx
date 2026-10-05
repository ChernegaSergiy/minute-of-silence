import { Avatar, AvatarProps } from "@fluentui/react-components";
import { useCachedImage } from "../hooks/useCachedImage";

interface CachedAvatarProps extends Omit<AvatarProps, "image"> {
  imageUrl: string;
}

export const CachedAvatar = ({ imageUrl, ...props }: CachedAvatarProps) => {
  const cachedUrl = useCachedImage(imageUrl);

  return (
    <Avatar
      {...props}
      image={{
        src: cachedUrl || imageUrl,
        onError: (e) => { e.currentTarget.style.display = 'none'; }
      }}
    />
  );
};
